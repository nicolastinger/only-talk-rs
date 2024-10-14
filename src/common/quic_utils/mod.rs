use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use quinn::{ClientConfig, Endpoint, ServerConfig};

pub(crate) mod quic_server;
pub(crate) mod quic_client;

/// 自定义证书验证器，将任何证书视为有效。
/// 注意：这种验证容易受到中间人攻击，但便于测试。
struct SkipServerVerification;

impl SkipServerVerification {
    fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

impl rustls::client::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::Certificate,
        _intermediates: &[rustls::Certificate],
        _server_name: &rustls::ServerName,
        _scts: &mut dyn Iterator<Item=&[u8]>,
        _ocsp_response: &[u8],
        _now: std::time::SystemTime,
    ) -> Result<rustls::client::ServerCertVerified, rustls::Error> {
        // 返回成功验证结果
        Ok(rustls::client::ServerCertVerified::assertion())
    }
}

/// 配置客户端使用的QUIC设置。
fn configure_client() -> ClientConfig {
    // 构建TLS配置，使用安全默认值，自定义证书验证器，不进行客户端认证
    let crypto = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_custom_certificate_verifier(SkipServerVerification::new())
        .with_no_client_auth();

    // 创建QUIC客户端配置
    ClientConfig::new(Arc::new(crypto))
}
/// 构造一个QUIC端点，配置为监听特定地址和端口上的传入连接。
///
/// ## 返回
///
/// - 一个QUIC连接的输入流
/// - 服务器证书序列化为DER格式
#[allow(unused)]
pub fn make_server_endpoint(bind_addr: SocketAddr) -> Result<(Endpoint, Vec<u8>), Box<dyn Error>> {
    // 配置服务器设置，包括生成自签名证书
    let (server_config, server_cert) = configure_server()?;
    // 创建服务器端点
    let endpoint = Endpoint::server(server_config, bind_addr)?;
    Ok((endpoint, server_cert))
}

/// 返回默认的服务器配置及其证书。
fn configure_server() -> Result<(ServerConfig, Vec<u8>), Box<dyn Error>> {
    // 生成自签名证书
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let cert_der = cert.serialize_der().unwrap();
    let priv_key = cert.serialize_private_key_der();
    let priv_key = rustls::PrivateKey(priv_key);
    let cert_chain = vec![rustls::Certificate(cert_der.clone())];

    // 创建服务器配置
    let mut server_config = ServerConfig::with_single_cert(cert_chain, priv_key)?;
    let transport_config = Arc::get_mut(&mut server_config.transport).unwrap();
    transport_config.max_concurrent_uni_streams(0_u8.into()); // 设置最大并发单向流数量

    Ok((server_config, cert_der))
}

/// QUIC协议的ALPN标识符。
#[allow(unused)]
pub const ALPN_QUIC_HTTP: &[&[u8]] = &[b"hq-29"];