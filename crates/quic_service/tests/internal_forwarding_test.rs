//! 内部 QUIC 双节点消息转发集成测试
//!
//! 起两个内网端点(节点 A / 节点 B),每个节点挂一个模拟用户设备,
//! 验证节点间互相转发消息(节点 A -> 节点 B 投递、节点 B -> 节点 A 投递)。
//! 不依赖外部服务(Redis/DB),仅使用本地回环网络。

// 测试代码中不使用 unwrap,失败时通过 expect 携带上下文
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use common::config_str::{PC_PLATFORM, REDIS_QUIC_SERVERS, REDIS_SPLIT};
use common::state::CoreState;
use common::utils::internal_quic_client::{make_internal_client_config, send_internal_quic_msg};
use common::utils::internal_quic_msg::{InternalQuicRequest, InternalQuicResponse, RequestSource};
use common::utils::message_types::MSG_TYPE_TEXT;
use common::utils::text_msg::TextQuicMsg;
use dashmap::DashMap;
use quic_service::models::quic_connection::{ConnectionType, QuicConnection};
use quic_service::{InternalQuicConfig, run_internal_server};
use quinn::{Connection, Endpoint, ServerConfig};
use rcgen::KeyPair;
use rustls::{Certificate, PrivateKey};
use tokio::sync::{mpsc, watch};
use tracing::info;

/// 初始化日志输出,默认 debug 级别,可用 RUST_LOG 环境变量覆盖。
/// 用于打印内部服务器/客户端的运行日志,便于排查集成测试失败原因。
fn init_logging() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("debug"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

/// 获取一个空闲 UDP 端口(绑定后立即释放,供后续绑定)
fn free_udp_addr() -> SocketAddr {
    std::net::UdpSocket::bind("127.0.0.1:0")
        .expect("绑定 UDP socket 失败")
        .local_addr()
        .expect("获取本地 UDP 地址失败")
}

/// 构造与生产代码一致的连接 key(platform + "QUIC:SERVER:" + uuid + ":" + text)
fn conn_key(platform: &str, user_uuid: &str) -> String {
    format!(
        "{}:{}{}{}{}",
        platform,
        REDIS_QUIC_SERVERS,
        user_uuid,
        REDIS_SPLIT,
        ConnectionType::Text
    )
    .to_uppercase()
}

/// 构造仅用于测试的 CoreState(Redis/DB 不连接,内部服务器对它们的失败不致命)
fn make_core() -> CoreState {
    let redis_pool = deadpool_redis::Config::from_url("redis://127.0.0.1:1")
        .create_pool(Some(deadpool_redis::Runtime::Tokio1))
        .expect("创建 Redis 连接池失败");
    CoreState { db: rbatis::RBatis::new(), redis: redis_pool }
}

/// 创建自签名证书的 QUIC 服务端点(与内部服务器相同的证书策略)
fn make_device_endpoint() -> Endpoint {
    let key_pair = KeyPair::generate().expect("生成密钥对失败");
    let params =
        rcgen::CertificateParams::new(vec!["localhost".to_string()]).expect("构建证书参数失败");
    let cert = params.self_signed(&key_pair).expect("生成自签名证书失败");
    let cert_chain = vec![Certificate(cert.der().to_vec())];
    let key = PrivateKey(key_pair.serialize_der());

    let mut server_config =
        ServerConfig::with_single_cert(cert_chain, key).expect("构建服务器 TLS 配置失败");
    let transport = Arc::get_mut(&mut server_config.transport).expect("获取传输配置失败");
    transport.max_idle_timeout(Some(Duration::from_secs(30).try_into().expect("转换空闲超时失败")));
    Endpoint::server(server_config, "127.0.0.1:0".parse().expect("解析监听地址失败"))
        .expect("启动 QUIC 服务端点失败")
}

/// 模拟用户设备:接受节点的连接,持续读取节点推送过来的单向上行流
async fn device_reader(endpoint: Endpoint, tx: mpsc::Sender<Vec<u8>>) {
    let conn = match endpoint.accept().await {
        Some(connecting) => match connecting.await {
            Ok(c) => c,
            Err(_) => return,
        },
        None => return,
    };
    loop {
        match conn.accept_uni().await {
            Ok(mut recv) => {
                let mut buf = Vec::new();
                let mut chunk = [0u8; 4096];
                loop {
                    match recv.read(&mut chunk).await {
                        Ok(Some(n)) => buf.extend_from_slice(&chunk[..n]),
                        Ok(None) => break,
                        Err(_) => return,
                    }
                }
                info!("设备收到投递消息: {} 字节", buf.len());
                if tx.send(buf).await.is_err() {
                    return;
                }
            }
            Err(_) => return,
        }
    }
}

/// 节点作为 QUIC 客户端连接到用户设备,返回服务端持有的连接(模拟用户上线)
async fn node_to_device(device_addr: SocketAddr) -> Connection {
    let mut endpoint = Endpoint::client("0.0.0.0:0".parse().expect("解析客户端地址失败"))
        .expect("创建 QUIC 客户端端点失败");
    endpoint.set_default_client_config(
        make_internal_client_config().expect("构建 QUIC 客户端配置失败"),
    );
    endpoint
        .connect(device_addr, "localhost")
        .expect("发起 QUIC 连接失败")
        .await
        .expect("QUIC 连接握手失败")
}

fn make_quic_connection(conn: Connection, user_uuid: &str) -> QuicConnection {
    QuicConnection {
        is_online: true,
        uuid: user_uuid.to_string(),
        connection_type: ConnectionType::Text,
        conn,
        create_time: 0,
        update_time: 0,
        ipv4addr: String::new(),
        ipv6addr: String::new(),
    }
}

/// 启动一个内网节点,返回其监听地址与关闭信号发送端
async fn start_node(
    core: CoreState,
    server_index: u32,
    connections: Arc<DashMap<String, QuicConnection>>,
) -> (SocketAddr, watch::Sender<bool>) {
    let addr = free_udp_addr();
    let config = InternalQuicConfig {
        bind_address: addr,
        server_name: format!("node-{}", server_index),
        server_index,
        node_address: addr.to_string(),
    };
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    tokio::spawn(run_internal_server(core, config, connections, shutdown_rx));
    (addr, shutdown_tx)
}

/// 发送内部请求,节点尚未就绪时重试
async fn send_with_retry(addr: SocketAddr, request: InternalQuicRequest) -> InternalQuicResponse {
    let mut last_err = None;
    for _ in 0..30 {
        match send_internal_quic_msg(addr, request.clone()).await {
            Ok(resp) => return resp,
            Err(e) => last_err = Some(e),
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("无法连接内部节点 {}: {:?}", addr, last_err)
}

fn make_request(target_user: &str, preferred_index: u32, payload: Vec<u8>) -> InternalQuicRequest {
    InternalQuicRequest {
        msg_type: MSG_TYPE_TEXT,
        payload,
        target_user: target_user.to_string(),
        preferred_index,
        platform: PC_PLATFORM.to_string(),
        source: RequestSource::QuicExternal,
        ttl: 3,
        close_after_delivery: false,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_internal_nodes_forward_messages_bidirectionally() {
    init_logging();

    // 用户设备端点:user-a 挂在节点 A,user-b 挂在节点 B
    let device_a = make_device_endpoint();
    let device_b = make_device_endpoint();
    let device_a_addr = device_a.local_addr().expect("获取 device_a 地址失败");
    let device_b_addr = device_b.local_addr().expect("获取 device_b 地址失败");
    info!("device_a 正在监听 {},device_b 正在监听 {}", device_a_addr, device_b_addr);

    let (msg_tx_a, mut msg_rx_a) = mpsc::channel(16);
    let (msg_tx_b, mut msg_rx_b) = mpsc::channel(16);
    let device_a_task = tokio::spawn(device_reader(device_a, msg_tx_a));
    let device_b_task = tokio::spawn(device_reader(device_b, msg_tx_b));

    // 节点连接各自用户设备(模拟用户上线)
    info!("正在连接 user-a 设备 / user-b 设备...");
    let conn_a_user = node_to_device(device_a_addr).await;
    let conn_b_user = node_to_device(device_b_addr).await;

    let connections_a = Arc::new(DashMap::new());
    connections_a
        .insert(conn_key(PC_PLATFORM, "user-a"), make_quic_connection(conn_a_user, "user-a"));
    let connections_b = Arc::new(DashMap::new());
    connections_b
        .insert(conn_key(PC_PLATFORM, "user-b"), make_quic_connection(conn_b_user, "user-b"));

    // 起两个内网节点
    let core = make_core();
    let (node_a_addr, shutdown_a) = start_node(core.clone(), 0, connections_a).await;
    let (node_b_addr, shutdown_b) = start_node(core.clone(), 1, connections_b).await;
    info!("内部节点 A (index=0) 已绑定 {},节点 B (index=1) 已绑定 {}", node_a_addr, node_b_addr);

    // 构造消息负载(与真实流程一致的 TextQuicMsg bincode 二进制)
    let payload = bincode::serialize(&TextQuicMsg {
        nano_id: "nano-1".to_string(),
        text_type: MSG_TYPE_TEXT,
        raw: b"hello from node A".to_vec(),
        recv_user: "user-b".to_string(),
        send_user: "user-a".to_string(),
        timestamp: 1_700_000_000_000,
    })
    .expect("序列化 TextQuicMsg 失败");
    info!("payload 大小 = {} 字节,正在转发 A -> B...", payload.len());

    // 1) 节点 A -> 节点 B:请求发往 B,由 B 投递给本机 user-b
    let resp = send_with_retry(node_b_addr, make_request("user-b", 1, payload.clone())).await;
    info!("A -> B 响应: status={} delivered={:?}", resp.status, resp.delivered);
    assert_eq!(resp.status, "ok");
    assert_eq!(resp.delivered, Some(true));

    let received_b = tokio::time::timeout(Duration::from_secs(3), msg_rx_b.recv())
        .await
        .expect("等待 device_b 接收消息超时")
        .expect("device_b 消息通道已关闭");
    info!("device_b 收到 {} 字节", received_b.len());
    assert_eq!(received_b, payload);

    // 2) 节点 B -> 节点 A:反向转发,由 A 投递给本机 user-a
    info!("正在转发 B -> A...");
    let resp = send_with_retry(node_a_addr, make_request("user-a", 0, payload.clone())).await;
    info!("B -> A 响应: status={} delivered={:?}", resp.status, resp.delivered);
    assert_eq!(resp.status, "ok");
    assert_eq!(resp.delivered, Some(true));

    let received_a = tokio::time::timeout(Duration::from_secs(3), msg_rx_a.recv())
        .await
        .expect("等待 device_a 接收消息超时")
        .expect("device_a 消息通道已关闭");
    info!("device_a 收到 {} 字节", received_a.len());
    assert_eq!(received_a, payload);

    // 3) 目标用户不在本机 -> 返回用户离线
    let resp = send_with_retry(node_a_addr, make_request("offline-user", 0, payload.clone())).await;
    info!("离线目标响应: status={} delivered={:?}", resp.status, resp.delivered);
    assert_eq!(resp.status, "ok");
    assert_eq!(resp.delivered, Some(false));

    // 清理
    info!("正在关闭节点和设备端点");
    let _ = shutdown_a.send(true);
    let _ = shutdown_b.send(true);
    device_a_task.abort();
    device_b_task.abort();
}
