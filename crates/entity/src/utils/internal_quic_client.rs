use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use quinn::{ClientConfig, Endpoint, TransportConfig};
use rustls::client::{ServerCertVerified, ServerCertVerifier};
use rustls::{Certificate, ServerName};
use tracing::{error, info};

use crate::models::internal_quic_msg::{InternalQuicRequest, InternalQuicResponse};

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

/// 创建不校验服务端证书的 QUIC 客户端配置
fn make_internal_client_config() -> Result<ClientConfig> {
    let crypto = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
        .with_no_client_auth();

    let mut transport = TransportConfig::default();
    transport.max_idle_timeout(Some(Duration::from_secs(30).try_into()?));
    let mut config = ClientConfig::new(Arc::new(crypto));
    config.transport_config(Arc::new(transport));

    Ok(config)
}

/// 向内部 QUIC 服务发送请求并获取响应
///
/// 连接 → 发送请求 → 读取响应 → 关闭连接
pub async fn send_internal_quic_msg(
    server_addr: SocketAddr,
    request: InternalQuicRequest,
) -> Result<InternalQuicResponse> {
    let client_config = make_internal_client_config()?;
    let mut endpoint = Endpoint::client("0.0.0.0:0".parse()?)?;
    endpoint.set_default_client_config(client_config);

    info!("[内网QUIC客户端] 连接内网QUIC服务 {}", server_addr);

    let connection = endpoint.connect(server_addr, "localhost")?.await?;

    let (mut send_stream, mut recv_stream) = connection.open_bi().await?;

    // 发送请求
    let body = serde_json::to_string(&request)?;
    send_stream.write_all(body.as_bytes()).await?;
    send_stream.finish().await?;

    // 读取响应
    let mut buf = vec![0u8; 1024 * 16];
    match recv_stream.read(&mut buf).await? {
        Some(len) => {
            let resp: InternalQuicResponse = serde_json::from_slice(&buf[..len])?;
            info!("[内网QUIC客户端] 收到响应 status={}", resp.status);
            Ok(resp)
        }
        None => {
            error!("[内网QUIC客户端] 服务端关闭了流，未返回响应");
            Ok(InternalQuicResponse::error("服务端未返回响应"))
        }
    }
}
