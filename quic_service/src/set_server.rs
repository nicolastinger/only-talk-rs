use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use quinn::{ClientConfig, Endpoint, ServerConfig, TransportConfig};
use rustls::{Certificate, PrivateKey, RootCertStore};
use rustls_pemfile::{certs, ec_private_keys, rsa_private_keys ,pkcs8_private_keys};
use std::io::{Seek, SeekFrom};

/// 配置客户端使用的QUIC设置。
#[allow(dead_code)]
pub fn configure_client() -> ClientConfig {
    // 构建TLS配置，使用安全默认值，信任系统证书库
    let mut root_store = RootCertStore::empty();


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
pub fn configure_server() -> Result<(ServerConfig, Vec<u8>), Box<dyn Error>> {
    // 从.pem文件加载证书
    let mut cert_file =
        BufReader::new(File::open("./config/ssl/fullchain.pem").expect("打开pem文件失败"));
    let cert_chain: Vec<Certificate> = certs(&mut cert_file)
        .map(|certs| certs.into_iter().map(Certificate).collect())
        .map_err(|_| "无法解析证书文件")?;

    // 从.key文件加载私钥
     let key_file =
        &mut BufReader::new(File::open("./config/ssl/privkey.pem").expect("找不到TLS证书密钥"));

    // 尝试读取不同类型的私钥
    let mut keys = {
        // 读取RSA私钥
        key_file.seek(SeekFrom::Start(0)).expect("无法重置文件读取位置");
        if let Ok(keys) = rsa_private_keys(key_file) {
            if !keys.is_empty() {
                keys
            } else {
                // 读取EC私钥
                key_file.seek(SeekFrom::Start(0)).expect("无法重置文件读取位置");
                if let Ok(keys) = ec_private_keys(key_file) {
                    if !keys.is_empty() {
                        keys
                    } else {
                        // 读取PKCS8私钥
                        key_file.seek(SeekFrom::Start(0)).expect("无法重置文件读取位置");
                        pkcs8_private_keys(key_file).expect("无法读取私钥")
                    }
                } else {
                    // 读取PKCS8私钥
                    key_file.seek(SeekFrom::Start(0)).expect("无法重置文件读取位置");
                    pkcs8_private_keys(key_file).expect("无法读取私钥")
                }
            }
        } else {
            // 读取EC私钥
            key_file.seek(SeekFrom::Start(0)).expect("无法重置文件读取位置");
            if let Ok(keys) = ec_private_keys(key_file) {
                if !keys.is_empty() {
                    keys
                } else {
                    // 读取PKCS8私钥
                    key_file.seek(SeekFrom::Start(0)).expect("无法重置文件读取位置");
                    pkcs8_private_keys(key_file).expect("无法读取私钥")
                }
            } else {
                // 读取PKCS8私钥
                key_file.seek(SeekFrom::Start(0)).expect("无法重置文件读取位置");
                pkcs8_private_keys(key_file).expect("无法读取私钥")
            }
        }
    };
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
    transport_config.max_idle_timeout(Some(Duration::from_secs(190).try_into().unwrap())); //最大容忍三次连接超时
    // 返回服务器配置和证书
    Ok((server_config, cert_der))
}
