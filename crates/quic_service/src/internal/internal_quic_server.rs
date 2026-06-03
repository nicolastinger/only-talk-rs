use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use dashmap::DashMap;
use deadpool_redis::redis::AsyncCommands;
use common::config_str::REDIS_INTERNAL_QUIC_SERVERS;
use common::config_str::{REDIS_QUIC_SERVERS, REDIS_SPLIT};
use common::utils::group_msg::{InternalGroupBroadcast, InternalGroupBroadcastResponse};
use common::utils::internal_quic_msg::{InternalQuicRequest, InternalQuicResponse};
use common::REDIS_CLIENT;
use quinn::{Endpoint, RecvStream, SendStream, ServerConfig};
use rcgen::KeyPair;
use rustls::{Certificate, PrivateKey};
use tokio::sync::watch;
use tracing::{debug, error, info, warn};

use super::internal_config::InternalQuicConfig;
use crate::models::quic_connection::{ConnectionType, QuicConnection};
use crate::msg_service::group_msg_service::process_group_broadcast;

fn generate_self_signed_cert() -> Result<(Vec<Certificate>, PrivateKey), Box<dyn std::error::Error>> {
    let key_pair = KeyPair::generate()?;
    let params = rcgen::CertificateParams::new(vec!["localhost".to_string()])?;
    let cert = params.self_signed(&key_pair)?;
    let cert_der = cert.der().to_vec();
    let key_der = key_pair.serialize_der();
    Ok((vec![Certificate(cert_der)], PrivateKey(key_der)))
}

fn make_internal_endpoint(bind_addr: SocketAddr) -> Result<Endpoint, Box<dyn std::error::Error>> {
    let (cert_chain, key) = generate_self_signed_cert()?;
    let mut server_config = ServerConfig::with_single_cert(cert_chain, key)?;
    let transport = Arc::get_mut(&mut server_config.transport)
        .ok_or_else(|| "获取传输配置失败")?;
    transport.max_concurrent_uni_streams(32_u8.into());
    transport.max_concurrent_bidi_streams(32_u8.into());
    transport.max_idle_timeout(Some(Duration::from_secs(30).try_into()?));
    let endpoint = Endpoint::server(server_config, bind_addr)?;
    Ok(endpoint)
}

async fn handle_internal_request(
    mut send_stream: SendStream,
    mut recv_stream: RecvStream,
    connections: Arc<DashMap<String, QuicConnection>>,
    server_index: u32,
) -> Result<()> {
    info!("[internal QUIC server] received new request, server_index={}, reading data...", server_index);

    let mut buf = vec![0u8; 1024 * 64];
    match recv_stream.read(&mut buf).await? {
        Some(len) => {
            info!("[internal QUIC server] read request size={} bytes", len);

            // 尝试解析为群聊广播
            if let Ok(group_req) = bincode::deserialize::<InternalGroupBroadcast>(&buf[..len]) {
                info!(
                    "[internal QUIC server] detected group chat broadcast group_uuid={} sender={}",
                    group_req.group_uuid,
                    group_req.sender
                );
                // 群聊广播处理
                let resp = match process_group_broadcast(&group_req, &connections).await {
                    Ok(_) => bincode::serialize(&InternalGroupBroadcastResponse::ok())?,
                    Err(e) => {
                        error!("[internal QUIC server] failed to process group chat broadcast: {}", e);
                        bincode::serialize(&InternalGroupBroadcastResponse::error(e.to_string()))?
                    }
                };
                send_stream.write_all(&resp).await?;
                send_stream.finish().await?;
                info!("[internal QUIC server] group chat broadcast response sent");
                return Ok(());
            }

            // 尝试解析为文本消息请求（直接本机投递，不再跨节点路由）
            if let Ok(request) = bincode::deserialize::<InternalQuicRequest>(&buf[..len]) {
                info!(
                    "[internal QUIC server] detected text message request target_user={} msg_type={} platform={} preferred_index={} ttl={} source={:?}",
                    request.target_user, request.msg_type, request.platform, request.preferred_index, request.ttl, request.source
                );

                // 构造连接 key，直接在本机查找目标用户
                let connection_key = format!(
                    "{}:{}{}{}{}",
                    request.platform,
                    REDIS_QUIC_SERVERS,
                    request.target_user,
                    REDIS_SPLIT,
                    ConnectionType::Text
                );
                let connection_key = connection_key.to_uppercase();
                debug!("[internal QUIC server] looking up local connection key={}", connection_key);

                let response = match connections.get(&connection_key) {
                    Some(entry) => {
                        info!(
                            "[internal QUIC server] found target user {} locally, delivering...",
                            request.target_user
                        );
                        let conn = entry.conn.clone();

                        if let Err(e) = deliver_to_local_conn(conn, &request).await {
                            error!("[internal QUIC server] delivery failed: {}", e);
                            InternalQuicResponse::error(format!("投递失败: {}", e))
                        } else {
                            info!("[internal QUIC server] delivery successful target={}", request.target_user);
                            InternalQuicResponse::ok()
                        }
                    }
                    None => {
                        warn!(
                            "[internal QUIC server] target user not found locally key={} (user offline)",
                            connection_key
                        );
                        InternalQuicResponse::user_offline()
                    }
                };

                let resp_bytes = bincode::serialize(&response)?;
                info!(
                    "[internal QUIC server] response status={} delivered={:?} message={:?}",
                    response.status, response.delivered, response.message
                );

                send_stream.write_all(&resp_bytes).await?;
                send_stream.finish().await?;
                info!("[internal QUIC server] text message response sent, processing complete");
                return Ok(());
            }

            warn!("[internal QUIC server] unrecognized request format size={} bytes", len);
            let resp = InternalQuicResponse::error("无法识别的请求格式");
            send_stream.write_all(&bincode::serialize(&resp)?).await?;
            send_stream.finish().await?;
        }
        None => {
            warn!("[internal QUIC server] client closed stream, no data sent");
            send_stream.finish().await?;
        }
    }
    Ok(())
}

/// 向本机连接投递消息（直接透传，payload 已是 TextQuicMsg 二进制）
async fn deliver_to_local_conn(
    conn: quinn::Connection,
    request: &InternalQuicRequest,
) -> Result<()> {
    info!(
        "[internal QUIC server] starting delivery msg_type={} target_user={} payload_len={}",
        request.msg_type,
        request.target_user,
        request.payload.len()
    );

    let mut send = conn.open_uni().await?;
    debug!("[internal QUIC server] uni stream opened");

    // payload 已经是 bincode 序列化的 TextQuicMsg 二进制，直接透传给客户端
    send.write_all(&request.payload).await?;
    send.finish().await?;
    info!(
        "[internal QUIC server] delivery complete, passthrough {} bytes",
        request.payload.len()
    );
    Ok(())
}

async fn register_to_redis(config: &InternalQuicConfig) -> Result<()> {
    let redis = REDIS_CLIENT.read().await;
    let redis = redis
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("failed to get Redis connection pool"))?;
    let mut conn = redis.get().await?;
    let key = format!("{}{}", REDIS_INTERNAL_QUIC_SERVERS, config.server_index);
    let value = config.node_address.clone();
    conn.set_ex::<&str, &str, ()>(&key, &value, 7200).await?;
    info!(
        "[internal QUIC server] registered to Redis key={} value={} (TTL=7200s)",
        key, value
    );
    Ok(())
}

async fn unregister_from_redis(config: &InternalQuicConfig) {
    if let Ok(redis) = REDIS_CLIENT.try_read() {
        if let Some(redis) = redis.as_ref() {
            if let Ok(mut conn) = redis.get().await {
                let key = format!("{}{}", REDIS_INTERNAL_QUIC_SERVERS, config.server_index);
                let _: Result<(), _> = conn.del(&key).await;
                info!("[internal QUIC server] unregistered from Redis key={}", key);
            }
        }
    }
}

pub async fn run_internal_server(
    config: InternalQuicConfig,
    connections: Arc<DashMap<String, QuicConnection>>,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    info!(
        "[internal QUIC server] initializing... bind_address={} server_index={} node_address={}",
        config.bind_address,
        config.server_index,
        config.node_address
    );

    let endpoint = match make_internal_endpoint(config.bind_address) {
        Ok(ep) => ep,
        Err(e) => {
            error!("[internal QUIC server] failed to create endpoint: {}", e);
            return;
        }
    };

    if let Err(e) = register_to_redis(&config).await {
        warn!("[internal QUIC server] failed to register to Redis (non-fatal): {}", e);
    }

    let server_index = config.server_index;
    info!(
        "[internal QUIC server] service started, listening on: {}, index: {}",
        config.bind_address, server_index
    );

    loop {
        let incoming_conn = {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    info!("[internal QUIC server] received shutdown signal");
                    break;
                }
                result = endpoint.accept() => {
                    match result {
                        Some(conn) => {
                            debug!("[internal QUIC server] received new connection");
                            conn
                        }
                        None => {
                            error!("[internal QUIC server] endpoint closed");
                            break;
                        }
                    }
                }
            }
        };

        let conn = match incoming_conn.await {
            Ok(c) => {
                info!(
                    "[internal QUIC server] new connection established remote_addr={}",
                    c.remote_address()
                );
                c
            }
            Err(e) => {
                error!("[internal QUIC server] failed to establish connection: {}", e);
                continue;
            }
        };

        let conns = connections.clone();
        tokio::spawn(async move {
            match conn.accept_bi().await {
                Ok((send_stream, recv_stream)) => {
                    debug!("[internal QUIC server] bi-directional stream opened");
                    if let Err(e) =
                        handle_internal_request(send_stream, recv_stream, conns, server_index)
                            .await
                    {
                        error!("[internal QUIC server] request processing exception: {}", e);
                    }
                }
                Err(e) => {
                    error!("[internal QUIC server] failed to open bi-directional stream: {}", e);
                }
            }
        });
    }

    unregister_from_redis(&config).await;
    info!("[internal QUIC server] service shutdown");
}
