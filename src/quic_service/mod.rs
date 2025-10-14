use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use quinn::{ClientConfig, Endpoint, ServerConfig, TransportConfig};
use rustls::{Certificate, PrivateKey, RootCertStore};
use rustls_pemfile::{certs, ec_private_keys, rsa_private_keys};

pub(crate) mod quic_server;
pub(crate) mod quic_client;
pub mod models;
pub mod msg_service;
pub mod init_server;

/// 配置客户端使用的QUIC设置。
fn configure_client() -> ClientConfig {
    // 构建TLS配置，使用安全默认值，信任系统证书库
    let mut root_store = RootCertStore::empty();

    let mut cert_file2 = BufReader::new(File::open("config/TLS/DigiCertGlobalRootCA.crt.pem").expect("打开pem文件失败"));
    let ca_certs:Vec<Certificate> = certs(&mut cert_file2)
        .map(|certs| certs.into_iter().map(Certificate).collect())
        .map_err(|_| "无法解析证书文件").expect("解析失败");

    // 添加CA证书到根证书存储
    for cert in ca_certs {
        root_store.add(&cert).expect("存储证书失败");
    }

    let crypto = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    // 创建QUIC客户端配置
   let mut config = ClientConfig::new(Arc::new(crypto));
    let mut time_out_config = TransportConfig::default();
    time_out_config.max_idle_timeout(Some(Duration::from_secs(1800).try_into().unwrap()));
    // 获取传输配置并设置最大空闲超时时间（例如3分钟）
    config.transport_config(Arc::from(time_out_config));
    config
}
/// 构造一个QUIC端点，配置为监听特定地址和端口上的传入连接。
///
/// ## 返回
///
/// - 一个QUIC连接的输入流
/// - 服务器证书序列化为DER格式

pub fn make_server_endpoint(bind_addr: SocketAddr) -> Result<(Endpoint, Vec<u8>), Box<dyn Error>> {
    // 配置服务器设置，包括生成自签名证书
    let (server_config, server_cert) = configure_server()?;
    // 创建服务器端点
    let endpoint = Endpoint::server(server_config, bind_addr)?;
    Ok((endpoint, server_cert))
}

/// 返回默认的服务器配置及其证书。
fn configure_server() -> Result<(ServerConfig, Vec<u8>), Box<dyn Error>> {
    // 从.pem文件加载证书
    let mut cert_file = BufReader::new(File::open("config/TLS/onlytalk.cn.pem").expect("打开pem文件失败"));
    let cert_chain:Vec<Certificate> = certs(&mut cert_file)
        .map(|certs| certs.into_iter().map(Certificate).collect())
        .map_err(|_| "无法解析证书文件")?;

    // 从.key文件加载私钥
    let mut key_file = BufReader::new(File::open("config/TLS/onlytalk.cn.key").expect("打开key文件失败"));
    let mut keys = rsa_private_keys(&mut key_file).or_else(|_| ec_private_keys(&mut key_file))
        .map_err(|_| "无法解析私钥文件")?;
    if keys.is_empty() {
        return Err("私钥文件为空".into());
    }
    let key = PrivateKey(keys.remove(0));

    // 克隆第一个证书
    let cert_der = cert_chain.first().cloned().ok_or("证书链为空")?.0;

    // 创建服务器配置
    let mut server_config = ServerConfig::with_single_cert(cert_chain.clone(), key)?;
    let transport_config = Arc::get_mut(&mut server_config.transport).unwrap();
    transport_config.max_concurrent_uni_streams(0_u8.into()); // 设置最大并发单向流数量
    transport_config.max_idle_timeout(Some(Duration::from_secs(190).try_into().unwrap()));  //最大容忍三次连接超时
    // 返回服务器配置和证书
    Ok((server_config, cert_der))
}