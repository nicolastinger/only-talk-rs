use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use common::config_str::{REDIS_INTERNAL_QUIC_SERVERS, REDIS_QUIC_SERVERS, REDIS_SPLIT};
use common::state::CoreState;
use common::utils::group_msg::{InternalGroupBroadcast, InternalGroupBroadcastResponse};
use common::utils::internal_quic_msg::{InternalQuicRequest, InternalQuicResponse};
use common::utils::mask::mask_addr;
use dashmap::DashMap;
use deadpool_redis::redis::AsyncCommands;
use quinn::{Endpoint, RecvStream, SendStream, ServerConfig};
use rcgen::KeyPair;
use rustls::{Certificate, PrivateKey};
use tokio::sync::watch;
use tracing::{debug, error, info, warn};

use super::internal_config::InternalQuicConfig;
use crate::models::quic_connection::{ConnectionType, QuicConnection};
use crate::msg_service::group_msg_service::process_group_broadcast;

fn generate_self_signed_cert() -> Result<(Vec<Certificate>, PrivateKey), Box<dyn std::error::Error>>
{
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
    let transport =
        Arc::get_mut(&mut server_config.transport).ok_or("Failed to get transport config")?;
    transport.max_concurrent_uni_streams(32_u8.into());
    transport.max_concurrent_bidi_streams(32_u8.into());
    transport.max_idle_timeout(Some(Duration::from_secs(30).try_into()?));
    let endpoint = Endpoint::server(server_config, bind_addr)?;
    Ok(endpoint)
}

async fn handle_internal_request(
    core: &CoreState,
    mut send_stream: SendStream,
    mut recv_stream: RecvStream,
    connections: Arc<DashMap<String, QuicConnection>>,
    server_index: u32,
) -> Result<()> {
    info!("[内部 QUIC 服务器] 收到新请求,server_index={},正在读取数据...", server_index);

    let mut buf = vec![0u8; 1024 * 64];
    match recv_stream.read(&mut buf).await? {
        Some(len) => {
            info!("[内部 QUIC 服务器] 读取到请求,大小={} 字节", len);

            // 尝试解析为群聊广播
            if let Ok(group_req) = bincode::deserialize::<InternalGroupBroadcast>(&buf[..len]) {
                info!(
                    "[内部 QUIC 服务器] [群聊] 收到广播 group_uuid={} sender={}",
                    group_req.group_uuid, group_req.sender
                );
                // 群聊广播处理
                let resp = match process_group_broadcast(&group_req, &connections).await {
                    Ok(_) => bincode::serialize(&InternalGroupBroadcastResponse::ok())?,
                    Err(e) => {
                        error!("[内部 QUIC 服务器] [群聊] 处理广播失败: {}", e);
                        bincode::serialize(&InternalGroupBroadcastResponse::error(e.to_string()))?
                    }
                };
                send_stream.write_all(&resp).await?;
                send_stream.finish().await?;
                info!("[内部 QUIC 服务器] [群聊] 广播响应已发送");
                return Ok(());
            }

            // 尝试解析为文本消息请求(直接本地投递,不跨节点路由)
            if let Ok(request) = bincode::deserialize::<InternalQuicRequest>(&buf[..len]) {
                info!(
                    "[内部 QUIC 服务器] [单聊] 收到请求 target_user={} msg_type={} platform={} preferred_index={} ttl={} source={:?}",
                    request.target_user,
                    request.msg_type,
                    request.platform,
                    request.preferred_index,
                    request.ttl,
                    request.source
                );

                // 构造连接 key,在本机查找目标用户
                let connection_key = format!(
                    "{}:{}{}{}{}",
                    request.platform,
                    REDIS_QUIC_SERVERS,
                    request.target_user,
                    REDIS_SPLIT,
                    ConnectionType::Text
                );
                let connection_key = connection_key.to_uppercase();
                debug!("[内部 QUIC 服务器] [单聊] 正在查找本地连接 key={}", connection_key);

                let response = match connections.get(&connection_key) {
                    Some(entry) => {
                        info!(
                            "[内部 QUIC 服务器] [单聊] 在本机找到目标用户 {},正在投递...",
                            request.target_user
                        );
                        let conn = entry.conn.clone();

                        let result = if request.close_after_delivery {
                            kick_local_connection(
                                core,
                                &connections,
                                &connection_key,
                                conn,
                                &request,
                            )
                            .await
                        } else {
                            deliver_to_local_conn(conn, &request).await
                        };
                        if let Err(e) = result {
                            error!("[内部 QUIC 服务器] [单聊] 投递失败: {}", e);
                            InternalQuicResponse::error(format!("Delivery failed: {}", e))
                        } else {
                            info!(
                                "[内部 QUIC 服务器] [单聊] 投递成功 target={}",
                                request.target_user
                            );
                            InternalQuicResponse::ok()
                        }
                    }
                    None => {
                        warn!(
                            "[内部 QUIC 服务器] [单聊] 本机未找到目标用户 key={}(用户离线)",
                            connection_key
                        );
                        InternalQuicResponse::user_offline()
                    }
                };

                let resp_bytes = bincode::serialize(&response)?;
                info!(
                    "[内部 QUIC 服务器] [单聊] 响应 status={} delivered={:?} message={:?}",
                    response.status, response.delivered, response.message
                );

                send_stream.write_all(&resp_bytes).await?;
                send_stream.finish().await?;
                info!("[内部 QUIC 服务器] [单聊] 响应已发送,处理完成");
                return Ok(());
            }

            warn!("[内部 QUIC 服务器] 无法识别的请求格式,大小={} 字节", len);
            let resp = InternalQuicResponse::error("Unrecognized request format");
            send_stream.write_all(&bincode::serialize(&resp)?).await?;
            send_stream.finish().await?;
        }
        None => {
            warn!("[内部 QUIC 服务器] 客户端已关闭流,未发送数据");
            send_stream.finish().await?;
        }
    }
    Ok(())
}

/// 将消息投递给本地连接(直接透传,payload 已是 TextQuicMsg 二进制)
async fn deliver_to_local_conn(
    conn: quinn::Connection,
    request: &InternalQuicRequest,
) -> Result<()> {
    info!(
        "[内部 QUIC 服务器] [单聊] 开始投递 msg_type={} target_user={} payload_len={}",
        request.msg_type,
        request.target_user,
        request.payload.len()
    );

    let mut send = conn.open_uni().await?;
    debug!("[内部 QUIC 服务器] [单聊] 单向流已打开");

    // payload 已经是 bincode 序列化的 TextQuicMsg 二进制，直接透传给客户端
    send.write_all(&request.payload).await?;
    send.finish().await?;
    info!("[内部 QUIC 服务器] [单聊] 投递完成,透传 {} 字节", request.payload.len());
    Ok(())
}

async fn kick_local_connection(
    core: &CoreState,
    connections: &Arc<DashMap<String, QuicConnection>>,
    connection_key: &str,
    conn: quinn::Connection,
    request: &InternalQuicRequest,
) -> Result<()> {
    let mut send = conn.open_uni().await?;
    send.write_all(&request.payload).await?;
    send.finish().await?;
    conn.close(0u32.into(), b"replaced by another login");

    if connections
        .get(connection_key)
        .map(|entry| entry.conn.stable_id() == conn.stable_id())
        .unwrap_or(false)
    {
        connections.remove(connection_key);
        let mut redis = core.redis.get().await?;
        let _: () = redis.del(connection_key).await?;
    }
    Ok(())
}

async fn register_to_redis(core: &CoreState, config: &InternalQuicConfig) -> Result<()> {
    let mut conn = core.redis.get().await?;
    let key = format!("{}{}", REDIS_INTERNAL_QUIC_SERVERS, config.server_index);
    let value = config.node_address.clone();
    conn.set_ex::<&str, &str, ()>(&key, &value, 7200).await?;
    info!("[内部 QUIC 服务器] 已注册到 Redis key={} value={} (TTL=7200s)", key, mask_addr(&value));
    Ok(())
}

async fn unregister_from_redis(core: &CoreState, config: &InternalQuicConfig) {
    if let Ok(mut conn) = core.redis.get().await {
        let key = format!("{}{}", REDIS_INTERNAL_QUIC_SERVERS, config.server_index);
        let _: Result<(), _> = conn.del(&key).await;
        info!("[内部 QUIC 服务器] 已从 Redis 注销 key={}", key);
    }
}

pub async fn run_internal_server(
    core: CoreState,
    config: InternalQuicConfig,
    connections: Arc<DashMap<String, QuicConnection>>,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    info!(
        "[内部 QUIC 服务器] 正在初始化... bind_address={} server_index={} node_address={}",
        config.bind_address,
        config.server_index,
        mask_addr(&config.node_address)
    );

    let endpoint = match make_internal_endpoint(config.bind_address) {
        Ok(ep) => ep,
        Err(e) => {
            error!("[内部 QUIC 服务器] 创建 endpoint 失败: {}", e);
            return;
        }
    };

    if let Err(e) = register_to_redis(&core, &config).await {
        warn!("[内部 QUIC 服务器] 注册到 Redis 失败(非致命): {}", e);
    }

    let server_index = config.server_index;
    info!("[内部 QUIC 服务器] 服务已启动,监听地址: {},索引: {}", config.bind_address, server_index);

    loop {
        let incoming_conn = {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    info!("[内部 QUIC 服务器] 收到关闭信号");
                    break;
                }
                result = endpoint.accept() => {
                    match result {
                        Some(conn) => {
                            debug!("[内部 QUIC 服务器] 收到新连接");
                            conn
                        }
                        None => {
                            error!("[内部 QUIC 服务器] endpoint 已关闭");
                            break;
                        }
                    }
                }
            }
        };

        let conn = match incoming_conn.await {
            Ok(c) => {
                info!(
                    "[内部 QUIC 服务器] 新连接已建立 remote_addr={}",
                    mask_addr(&c.remote_address().to_string())
                );
                c
            }
            Err(e) => {
                error!("[内部 QUIC 服务器] 建立连接失败: {}", e);
                continue;
            }
        };

        let conns = connections.clone();
        let core_clone = core.clone();
        tokio::spawn(async move {
            match conn.accept_bi().await {
                Ok((send_stream, recv_stream)) => {
                    info!("[内部 QUIC 服务器] 双向流已打开");
                    if let Err(e) = handle_internal_request(
                        &core_clone,
                        send_stream,
                        recv_stream,
                        conns,
                        server_index,
                    )
                    .await
                    {
                        error!("[内部 QUIC 服务器] 请求处理异常: {}", e);
                    }
                }
                Err(e) => {
                    error!("[内部 QUIC 服务器] 打开双向流失败: {}", e);
                }
            }
        });
    }

    unregister_from_redis(&core, &config).await;
    info!("[内部 QUIC 服务器] 服务已关闭");
}
