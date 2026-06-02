use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use dashmap::DashMap;
use once_cell::sync::Lazy;
use quinn::{ClientConfig, Connection, Endpoint, TransportConfig};
use rustls::client::{ServerCertVerified, ServerCertVerifier};
use rustls::{Certificate, ServerName};
use tracing::info;

use crate::utils::internal_quic_msg::{InternalQuicRequest, InternalQuicResponse};

/// 跳过服务端证书校验 (内网服务使用自签名证书)
#[derive(Debug)]
struct SkipServerVerification;

impl ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &Certificate,
        _intermediates: &[Certificate],
        _server_name: &ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp_response: &[u8],
        _now: std::time::SystemTime,
    ) -> std::result::Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }
}

fn make_client_config() -> Result<ClientConfig> {
    let crypto = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
        .with_no_client_auth();

    let mut transport = TransportConfig::default();
    transport.max_idle_timeout(Some(Duration::from_secs(300).try_into()?));
    let mut config = ClientConfig::new(Arc::new(crypto));
    config.transport_config(Arc::new(transport));
    Ok(config)
}

/// 内网 QUIC 连接池，复用跨节点连接
pub struct InternalQuicClientPool {
    endpoint: Endpoint,
    connections: DashMap<SocketAddr, Connection>,
}

impl InternalQuicClientPool {
    pub fn new() -> Result<Self> {
        let client_config = make_client_config()?;
        let mut endpoint = Endpoint::client("0.0.0.0:0".parse()?)?;
        endpoint.set_default_client_config(client_config);
        Ok(Self {
            endpoint,
            connections: DashMap::new(),
        })
    }

    /// 获取或建立到目标节点的连接
    pub async fn get_or_connect(&self, addr: SocketAddr) -> Result<Connection> {
        // 快速路径：已有健康连接
        if let Some(conn) = self.connections.get(&addr) {
            if conn.close_reason().is_none() {
                return Ok(conn.clone());
            }
        }

        // 移除死连接
        if let Some((dead_addr, _)) = self.connections.remove(&addr) {
            info!("[内网QUIC连接池] 移除死连接 {}", dead_addr);
        }

        // 建立新连接
        let conn = self
            .endpoint
            .connect(addr, "localhost")?
            .await
            .with_context(|| format!("连接内网QUIC服务 {} 失败", addr))?;

        info!("[内网QUIC连接池] 新建连接 {}", addr);
        self.connections.insert(addr, conn.clone());
        Ok(conn)
    }

    /// 预热：主动建立到指定地址的连接
    pub async fn warmup(&self, addrs: Vec<SocketAddr>) {
        for addr in addrs {
            let _ = self.get_or_connect(addr).await;
        }
    }
}

/// 全局内网 QUIC 连接池
static INTERNAL_QUIC_POOL: Lazy<Result<Arc<InternalQuicClientPool>, String>> =
    Lazy::new(|| InternalQuicClientPool::new().map(Arc::new).map_err(|e| e.to_string()));

pub fn get_pool() -> &'static Arc<InternalQuicClientPool> {
    INTERNAL_QUIC_POOL
        .as_ref()
        .expect("内网QUIC连接池初始化失败")
}

/// 通过连接池发送内网 QUIC 消息
pub async fn send_internal_quic_msg(
    server_addr: SocketAddr,
    request: InternalQuicRequest,
) -> Result<InternalQuicResponse> {
    let pool = get_pool();
    let conn = pool.get_or_connect(server_addr).await?;

    let (mut send, mut recv) = conn.open_bi().await?;

    let body = bincode::serialize(&request)?;
    send.write_all(&body).await?;
    send.finish().await?;

    let mut buf = vec![0u8; 1024 * 16];
    match recv.read(&mut buf).await? {
        Some(len) => {
            let resp: InternalQuicResponse = bincode::deserialize(&buf[..len])?;
            Ok(resp)
        }
        None => Ok(InternalQuicResponse::error("服务端未返回响应")),
    }
}
