use log::{error, info, warn, LevelFilter};
use std::{error::Error, net::SocketAddr, sync::Arc};
use std::time::Duration;
use fast_log::Config;
use quinn::{ClientConfig, Endpoint, ServerConfig};

// 主函数入口点，使用Tokio异步运行时
#[tokio::main]
async fn main() {
    fast_log::init(Config::new().console().level(LevelFilter::Info).file("target/test.log").chan_len(Some(100000))).unwrap();

    info!("logging init");
    warn!("初始化日志警告信息");
    error!("初始化日志错误信息");
    // 定义服务器监听地址
    let addr = "127.0.0.1:4433".parse().unwrap();


    let res = tokio::spawn(run_server(addr));
    run_client(addr).await.expect("TODO: panic message");

    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    run_client(addr).await.expect("TODO: panic message");
    // 保持服务器运行
    res.await.expect("TODO: panic message");
}

/// 启动并运行QUIC服务器，持续监听新连接
async fn run_server(addr: SocketAddr) {
    // 创建服务器端点和证书
    let (endpoint, _server_cert) = make_server_endpoint(addr).unwrap();

    // 持续监听新的连接请求
    loop {
        let incoming_conn = endpoint.accept().await.unwrap(); // 接收新的连接请求
        let conn = incoming_conn.await.unwrap(); // 确认连接建立

        info!(
            "[服务端] 连接已接受: 地址={}",
            conn.remote_address() // 打印客户端地址
        );
        // 异步处理每个连接
        handle_conn(conn).await;
        info!("处理完成")
    }
}

async fn handle_conn(conn: quinn::Connection) {

    // 打开双向流
    let (mut send_stream, mut recv_stream) = match conn.accept_bi().await {
        Ok((send, recv)) => (send, recv),
        Err(e) => {
            error!("打开双向流失败: {}", e);
            return;
        }
    };

    info!("[server] 流已接受: ID={}", recv_stream.id()); // 打印流ID

    // 异步处理流中的数据
    let mut buffer = vec![0u8; 1024 * 8];
    loop {
        match recv_stream.read(&mut buffer).await {
            Ok(Some(length)) => {
                info!("[服务端] 长度为 {} 流数据: {:?}", length, String::from_utf8_lossy(&buffer[0..length]));
                send_stream.write_all("你在干什么".as_bytes()).await.expect("TODO: panic message");
            }
            Ok(None) => {
                info!("[服务端] 流关闭");
                break;
            }
            Err(e) => {
                error!("[服务端] 读取错误: {}", e);
                break;
            }
        }
    }
    info!("[服务端] 已结束运行");
}

/// 客户端异步函数，尝试与服务器建立QUIC连接
async fn run_client(server_addr: SocketAddr) -> Result<(), Box<dyn Error>> {
    // 创建客户端端点
    let mut endpoint = Endpoint::client("127.0.0.1:0".parse().unwrap())?;
    endpoint.set_default_client_config(configure_client()); // 设置默认客户端配置

    // 尝试连接到服务器
    let connection = endpoint.connect(server_addr, "localhost").unwrap().await.unwrap();
    info!("[client] connected: addr={}", connection.remote_address()); // 打印连接成功的服务器地址

    // 开启两个双向流
    let (mut send_stream1, mut _recv_stream) = connection.open_bi().await?;
    send_stream1.set_priority(0)?; // 设置优先级

    // 异步处理流中的数据
    tokio::spawn(async move {
        let mut buffer = vec![0u8; 1024 * 8];
        loop {
            match _recv_stream.read(&mut buffer).await {
                Ok(Some(length)) => {
                    info!("[客户端] 长度为 {} 流数据: {:?}", length, String::from_utf8_lossy(&buffer[0..length]));
                }
                Ok(None) => {
                    info!("[客户端]没有接收到数据")
                }
                Err(e) => {
                    error!("[客户端] 读取错误: {}", e);
                    break;
                }
            }
        }
    });
    // 发送消息给服务器
    send_stream1.write("我是谁".as_bytes()).await.unwrap();

    send_stream1.write("我是蔡徐坤".as_bytes()).await.unwrap();

    tokio::time::sleep(Duration::from_secs(1)).await;
    send_stream1.finish().await?;

    tokio::time::sleep(Duration::from_secs(1)).await;
    // 等待所有任务完成
    endpoint.wait_idle().await;

    Ok(())
}

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