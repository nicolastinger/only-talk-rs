use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use quinn::{ClientConfig, Endpoint, TransportConfig};
use rustls::client::{ServerCertVerified, ServerCertVerifier};
use rustls::{Certificate, ServerName};
use tracing::{debug, error, info, warn};

use crate::utils::internal_quic_msg::{InternalQuicRequest, InternalQuicResponse};

/// 跳过服务器证书校验(内部服务使用自签名证书)
#[derive(Debug)]
pub struct SkipServerVerification;

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

/// 创建跳过服务器证书校验的 QUIC 客户端配置
pub fn make_internal_client_config() -> Result<ClientConfig> {
    debug!("[内部 QUIC 客户端] [单聊] 正在创建客户端配置(跳过证书校验)");
    let crypto = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
        .with_no_client_auth();

    let mut transport = TransportConfig::default();
    transport.max_idle_timeout(Some(Duration::from_secs(30).try_into()?));
    let mut config = ClientConfig::new(Arc::new(crypto));
    config.transport_config(Arc::new(transport));

    debug!("[内部 QUIC 客户端] [单聊] 客户端配置完成 idle_timeout=30s");
    Ok(config)
}

/// 向内部 QUIC 服务发送请求并接收响应
///
/// 连接 -> 发送请求 -> 读取响应 -> 关闭连接
pub async fn send_internal_quic_msg(
    server_addr: SocketAddr,
    request: InternalQuicRequest,
) -> Result<InternalQuicResponse> {
    info!(
        "[内部 QUIC 客户端] [单聊] 发送内部请求 target_user={} msg_type={} preferred_index={}",
        request.target_user, request.msg_type, request.preferred_index
    );

    // 创建客户端配置
    let client_config = make_internal_client_config()?;
    let mut endpoint = Endpoint::client("0.0.0.0:0".parse()?)?;
    endpoint.set_default_client_config(client_config);
    debug!("[内部 QUIC 客户端] [单聊] 客户端端点已创建");

    // 建立连接
    info!("[内部 QUIC 客户端] [单聊] 正在连接内部节点 {}", server_addr);
    let connection = endpoint.connect(server_addr, "localhost")?.await.map_err(|e| {
        error!("[内部 QUIC 客户端] [单聊] 连接内部节点 {} 失败: {}", server_addr, e);
        anyhow::anyhow!("Internal QUIC connection to {} failed: {}", server_addr, e)
    })?;
    info!("[内部 QUIC 客户端] [单聊] 已连接内部节点 {}", server_addr);

    // 打开双向流
    let (mut send_stream, mut recv_stream) = connection.open_bi().await?;
    debug!("[内部 QUIC 客户端] [单聊] 双向流已打开");

    // 序列化并发送请求
    let body = bincode::serialize(&request)?;
    let body_len = body.len();
    debug!("[内部 QUIC 客户端] [单聊] 请求序列化完成,大小={} 字节", body_len);

    send_stream.write_all(&body).await?;
    debug!("[内部 QUIC 客户端] [单聊] 请求已发送 {} 字节", body_len);
    send_stream.finish().await?;
    debug!("[内部 QUIC 客户端] [单聊] 发送流已关闭");

    // 读取响应
    let mut buf = vec![0u8; 1024 * 16];
    debug!("[内部 QUIC 客户端] [单聊] 正在等待响应...");
    match recv_stream.read(&mut buf).await? {
        Some(len) => {
            debug!("[内部 QUIC 客户端] [单聊] 收到响应,大小={} 字节", len);
            let resp: InternalQuicResponse = bincode::deserialize(&buf[..len])?;
            info!(
                "[内部 QUIC 客户端] [单聊] 响应解析完成 status={} delivered={:?} message={:?}",
                resp.status, resp.delivered, resp.message
            );
            Ok(resp)
        }
        None => {
            warn!("[内部 QUIC 客户端] [单聊] 服务器关闭了流,未返回响应");
            Ok(InternalQuicResponse::error("server returned no response"))
        }
    }
}
