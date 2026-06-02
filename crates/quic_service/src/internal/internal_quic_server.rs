use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use dashmap::DashMap;
use deadpool_redis::redis::AsyncCommands;
use common::config_str::REDIS_INTERNAL_QUIC_SERVERS;
use common::utils::internal_quic_msg::InternalQuicResponse;
use common::REDIS_CLIENT;
use quinn::{Endpoint, RecvStream, SendStream, ServerConfig};
use rcgen::KeyPair;
use rustls::{Certificate, PrivateKey};
use tokio::sync::watch;
use tracing::{error, info};

use super::internal_config::InternalQuicConfig;
use super::internal_router::route_internal_request;
use crate::models::quic_connection::QuicConnection;

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
    transport.max_concurrent_uni_streams(200_u32.into());
    transport.max_concurrent_bidi_streams(200_u32.into());
    transport.max_idle_timeout(Some(Duration::from_secs(300).try_into()?));
    let endpoint = Endpoint::server(server_config, bind_addr)?;
    Ok(endpoint)
}

async fn handle_internal_request(
    mut send_stream: SendStream,
    mut recv_stream: RecvStream,
    connections: Arc<DashMap<String, QuicConnection>>,
    server_index: u32,
) -> Result<()> {
    let mut buf = vec![0u8; 1024 * 64];
    match recv_stream.read(&mut buf).await? {
        Some(len) => {
            let response = match route_internal_request(&buf[..len], &connections, server_index).await {
                Ok(resp) => resp,
                Err(e) => {
                    error!("[内网QUIC] 路由处理失败: {}", e);
                    serde_json::to_vec(&InternalQuicResponse::error(format!("路由处理失败: {}", e)))?
                }
            };

            send_stream.write_all(&response).await?;
            send_stream.finish().await?;
            info!("[内网QUIC] 处理完成");
        }
        None => {
            error!("[内网QUIC] 客户端关闭了流，未发送数据");
            send_stream.finish().await?;
        }
    }
    Ok(())
}

async fn register_to_redis(config: &InternalQuicConfig) -> Result<()> {
    let redis = REDIS_CLIENT.read().await;
    let redis = redis
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("获取Redis连接池失败"))?;
    let mut conn = redis.get().await?;
    let key = format!("{}{}", REDIS_INTERNAL_QUIC_SERVERS, config.server_index);
    let value = config.node_address.clone();
    conn.set_ex::<&str, &str, ()>(&key, &value, 7200).await?;
    info!(
        "[内网QUIC] 已注册到 Redis key={} value={}",
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
                info!("[内网QUIC] 已从 Redis 注销 key={}", key);
            }
        }
    }
}

pub async fn run_internal_server(
    config: InternalQuicConfig,
    connections: Arc<DashMap<String, QuicConnection>>,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    let endpoint = match make_internal_endpoint(config.bind_address) {
        Ok(ep) => ep,
        Err(e) => {
            error!("[内网QUIC] 创建端点失败: {}", e);
            return;
        }
    };

    if let Err(e) = register_to_redis(&config).await {
        tracing::warn!("[内网QUIC] 注册到 Redis 失败 (非致命): {}", e);
    }

    let server_index = config.server_index;
    info!(
        "[内网QUIC] 服务启动，监听地址: {}，序号: {}",
        config.bind_address, server_index
    );

    loop {
        let incoming_conn = {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    info!("[内网QUIC] 收到关闭信号");
                    break;
                }
                result = endpoint.accept() => {
                    match result {
                        Some(conn) => conn,
                        None => {
                            error!("[内网QUIC] endpoint 已关闭");
                            break;
                        }
                    }
                }
            }
        };

        let conn = match incoming_conn.await {
            Ok(c) => c,
            Err(e) => {
                error!("[内网QUIC] 建立连接失败: {}", e);
                continue;
            }
        };

        let conns = connections.clone();
        tokio::spawn(async move {
            match conn.accept_bi().await {
                Ok((send_stream, recv_stream)) => {
                    if let Err(e) =
                        handle_internal_request(send_stream, recv_stream, conns, server_index)
                            .await
                    {
                        error!("[内网QUIC] 处理请求失败: {}", e);
                    }
                }
                Err(e) => {
                    error!("[内网QUIC] 打开双向流失败: {}", e);
                }
            }
        });
    }

    unregister_from_redis(&config).await;
    info!("[内网QUIC] 服务已关闭");
}
