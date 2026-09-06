//! 外部 QUIC 服务器（ChatNode）集成测试
//!
//! 启动一个真实 ChatNode（自签名证书 + 测试 Redis），用 QUIC 客户端模拟用户设备，覆盖：
//! 1. 连接握手 + `FirstQuicMsg` JWT 鉴权 + 连接注册（内存 `connections` 映射 + Redis key）
//! 2. 心跳：客户端通过 uni 流发送 PING，服务器回推 PONG
//! 3. 登录接管三态(依据 JWT jti 的 uuid v7 签发时间):
//!    3a. 同一次登录重连(相同 jti)静默关闭旧连接,不发 FORCE_LOGOUT
//!    3b. 新登录(更新的 jti)强退旧连接,FORCE_LOGOUT 携带发起接管的新会话 jti
//!    3c. 过期登录(更老的 jti)重连被拒绝,在线会话保持不变
//! 4. 非法 token 被拒绝（服务器直接关闭流）
//! 5. 用户上线期间将 Redis 已读消息同步到 PostgreSQL
//! 6. 客户端断开后连接清理（`connections` 映射移除）
//!
//! 依赖：本地 Redis（`TEST_REDIS_URL`，建议独立 DB index）与仓库根目录 `.env`；
//! 需要 PostgreSQL（使用 `DATABASE_URL`，仅写入随机 UUID 测试数据）。
//!
//! 运行方式：
//!   cargo test -p quic_service --test external_server_integration_test -- --ignored

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use common::config_str::{PC_PLATFORM, PONG, SYSTEM, USER_READ_MSG};
use common::models::chat_entity::add_read_chat_record::AddReadChatRecordDTO;
use common::models::chat_entity::chat_message_read::ChatMessageRecordRead;
use common::models::chat_entity::chat_message_record::ChatMessageRecord;
use common::state::CoreState;
use common::utils::internal_quic_client::make_internal_client_config;
use common::utils::jwt_util::{generate_access_token, verify_token};
use common::utils::message_types::{MSG_TYPE_FORCE_LOGOUT, MSG_TYPE_PING};
use common::utils::text_msg::HeadMsg;
use deadpool_redis::redis::AsyncCommands;
use deadpool_redis::{Config as RedisConfig, Pool, Runtime};
use futures_util::FutureExt;
use quic_service::models::first_quic_msg::FirstQuicMsg;
use quic_service::models::quic_connection::ConnectionType;
use quic_service::msg_service::text_msg_service::get_text_msg;
use quic_service::{ChatNode, ChatNodeConfig, ServiceLifecycle};
use quinn::{Connection, Endpoint};
use rbatis::RBatis;
use rbatis::rbdc::Uuid as RbatisUuid;
use rbatis::rbdc::db::ConnectOptions;
use rbatis::rbdc::pool::{ConnectionManager, Pool as RbatisPool};
use rbdc_pg::PgDriver;
use rbdc_pg::options::PgConnectOptions;
use rbdc_pool_fast::FastPool;
use rsa::pkcs1::EncodeRsaPublicKey;
use rsa::pkcs8::EncodePrivateKey;
use tokio::sync::Mutex;
use tracing::info;
use uuid::Uuid;

/// 初始化日志输出，默认 info 级别，可用 RUST_LOG 覆盖
fn init_logging() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

/// 获取一个空闲 UDP 端口（绑定后立即释放，供后续绑定）
fn free_udp_addr() -> SocketAddr {
    std::net::UdpSocket::bind("127.0.0.1:0")
        .expect("绑定 UDP socket 失败")
        .local_addr()
        .expect("获取本地 UDP 地址失败")
}

/// HeadMsg 的 bincode 序列化长度（作为 FirstQuicMsg.dyn_header_size，与生产行为一致）
fn head_size() -> usize {
    bincode::serialize(&HeadMsg { version: 1, crc: 0, body_len: 0, message_type: 0 })
        .expect("序列化 HeadMsg 失败")
        .len()
}

/// 生成自签名证书并写入临时文件，返回 (cert_path, key_path)
fn make_cert_files(dir: &std::path::Path) -> (String, String) {
    let key_pair = rcgen::KeyPair::generate().expect("生成密钥对失败");
    let params =
        rcgen::CertificateParams::new(vec!["localhost".to_string()]).expect("构建证书参数失败");
    let cert = params.self_signed(&key_pair).expect("生成自签名证书失败");

    let cert_path = dir.join("quic_test_cert.pem");
    let key_path = dir.join("quic_test_key.pem");
    std::fs::write(&cert_path, cert.pem()).expect("写入证书文件失败");
    std::fs::write(&key_path, key_pair.serialize_pem()).expect("写入私钥文件失败");
    (cert_path.to_string_lossy().into_owned(), key_path.to_string_lossy().into_owned())
}

/// 在 config_manager 中注入 RSA 密钥，使 generate_access_token 与服务器 verify_token 使用同一对密钥
fn setup_jwt_keys() -> Result<()> {
    let mut rng = rand::thread_rng();
    let private_key = rsa::RsaPrivateKey::new(&mut rng, 2048).context("生成 RSA 私钥失败")?;
    let public_key = rsa::RsaPublicKey::from(&private_key);
    let private_key_pem =
        private_key.to_pkcs8_pem(Default::default()).context("私钥转 PEM 失败")?;
    let public_key_pem = public_key.to_pkcs1_pem(Default::default()).context("公钥转 PEM 失败")?;
    common::config_manager::set_config("jwt_private_key".to_string(), private_key_pem.to_string());
    common::config_manager::set_config("jwt_public_key".to_string(), public_key_pem.to_string());
    Ok(())
}

/// 构造仅用于测试的 CoreState（DB 不连接；本测试链路不触达数据库）
fn make_core(db: RBatis, redis_pool: Pool) -> CoreState {
    CoreState { db, redis: redis_pool }
}

async fn build_db_pool(url: &str) -> Result<RBatis> {
    let rb = RBatis::new();
    let mut options = PgConnectOptions::new();
    options.set_uri(url).context("设置 PostgreSQL URI 失败")?;
    let manager = ConnectionManager::new_options(PgDriver {}, options);
    let pool = FastPool::new(manager).context("创建 PostgreSQL 连接池失败")?;
    pool.set_timeout(Some(Duration::from_secs(5))).await;
    rb.pool.set(Box::new(pool)).map_err(|_| anyhow!("设置 PostgreSQL 连接池失败"))?;
    Ok(rb)
}

/// 构建 Redis 连接池
fn build_redis_pool(url: &str) -> Result<Pool> {
    let config = RedisConfig::from_url(url);
    let pool = config
        .create_pool(Some(Runtime::Tokio1))
        .map_err(|e| anyhow!("创建 Redis 连接池失败: {}", e))?;
    Ok(pool)
}

/// 清空测试 Redis 的所有 key
async fn flush_redis(pool: &Pool) -> Result<()> {
    let mut conn = pool.get().await.map_err(|e| anyhow!("获取 Redis 连接失败: {}", e))?;
    deadpool_redis::redis::cmd("FLUSHALL")
        .query_async::<()>(&mut conn)
        .await
        .map_err(|e| anyhow!("清空测试 Redis key 失败: {}", e))?;
    info!("测试 Redis 所有 key 已清空 (FLUSHALL)");
    Ok(())
}

/// QUIC 客户端端点（跳过自签名证书校验，与内部客户端一致）
async fn connect_client(server_addr: SocketAddr) -> Result<(Endpoint, Connection)> {
    let mut endpoint = Endpoint::client("0.0.0.0:0".parse().context("解析客户端地址失败")?)
        .context("创建 QUIC 客户端端点失败")?;
    endpoint.set_default_client_config(
        make_internal_client_config().context("构建 QUIC 客户端配置失败")?,
    );
    let conn = endpoint
        .connect(server_addr, "localhost")
        .context("发起 QUIC 连接失败")?
        .await
        .context("QUIC 连接握手失败")?;
    Ok((endpoint, conn))
}

/// 打开双向流并发送 FirstQuicMsg 初始化消息（保持 send 存活，使连接持续）
async fn send_first_msg(
    conn: &Connection,
    uuid: &str,
    token: &str,
    head_len: usize,
) -> Result<(quinn::SendStream, quinn::RecvStream)> {
    let (mut send, recv) = conn.open_bi().await.context("打开双向流失败")?;
    let first_msg = FirstQuicMsg {
        token: token.to_string(),
        uuid: uuid.to_string(),
        msg_type: ConnectionType::Text,
        text_serde_struct: String::new(),
        dyn_buffer_size: 0,
        dyn_header_size: head_len,
    };
    send.write_all(
        serde_json::to_string(&first_msg).context("序列化 FirstQuicMsg 失败")?.as_bytes(),
    )
    .await
    .context("发送 FirstQuicMsg 失败")?;
    Ok((send, recv))
}

/// 连接 key（与服务端 set_conn_info 的格式一致）
fn conn_key(platform: &str, user_uuid: &str) -> String {
    format!("{}:QUIC:SERVER:{}:{}", platform, user_uuid, ConnectionType::Text).to_uppercase()
}

/// 轮询等待 connections 映射中出现指定 key
async fn wait_connection_registered(node: &ChatNode, key: &str, timeout: Duration) -> Result<()> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if node.connections().contains_key(key) {
            return Ok(());
        }
        if std::time::Instant::now() > deadline {
            return Err(anyhow!("连接未在 {} 内注册: {}", timeout.as_secs(), key));
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// 轮询等待 connections 映射移除指定 key
async fn wait_connection_removed(node: &ChatNode, key: &str, timeout: Duration) -> Result<()> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if !node.connections().contains_key(key) {
            return Ok(());
        }
        if std::time::Instant::now() > deadline {
            return Err(anyhow!("连接未在 {} 内清理: {}", timeout.as_secs(), key));
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// 轮询等待连接映射离开指定 QUIC 连接
async fn wait_connection_changed(
    node: &ChatNode,
    key: &str,
    old_stable_id: usize,
    timeout: Duration,
) -> Result<()> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if let Some(entry) = node.connections().get(key)
            && entry.conn.stable_id() != old_stable_id
        {
            return Ok(());
        }
        if std::time::Instant::now() > deadline {
            return Err(anyhow!("连接未在 {} 内切换到新连接: {}", timeout.as_secs(), key));
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// 从连接上读取一枚服务端 uni 帧(返回完整帧字节);若连接在收到任何帧前被静默关闭则返回 None。
/// 用于区分"同会话静默顶替(无帧)"与"新登录强退(FORCE_LOGOUT 帧)"。
async fn recv_server_uni_or_closed(
    conn: &Connection,
    timeout: Duration,
) -> Result<Option<Vec<u8>>> {
    let result = tokio::time::timeout(timeout, async {
        tokio::select! {
            _ = conn.closed() => None,
            uni = conn.accept_uni() => match uni {
                Ok(mut recv) => {
                    let mut buf = Vec::new();
                    let mut chunk = [0u8; 4096];
                    while let Ok(Some(n)) = recv.read(&mut chunk).await {
                        buf.extend_from_slice(&chunk[..n]);
                    }
                    Some(buf)
                }
                Err(_) => None,
            }
        }
    })
    .await
    .map_err(|_| anyhow!("等待服务端 uni 帧/连接关闭超时"))?;
    Ok(result)
}

/// 从 FORCE_LOGOUT 帧的 raw(JSON)中提取发起强退的会话 jti
fn kick_session(raw: &[u8]) -> Option<String> {
    serde_json::from_slice::<serde_json::Value>(raw)
        .ok()
        .and_then(|value| value.get("session").and_then(|s| s.as_str()).map(|s| s.to_string()))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "需要本地 Redis 与仓库根目录 .env"]
async fn external_chat_node_connection_lifecycle() -> Result<()> {
    init_logging();
    dotenvy::dotenv().ok();
    let _ = dotenvy::from_path(concat!(env!("CARGO_MANIFEST_DIR"), "/../../.env"));

    let redis_url = std::env::var("TEST_REDIS_URL").map_err(|_| {
        anyhow!("未找到 TEST_REDIS_URL，请在仓库根目录 .env 中配置（建议独立 DB index，如 redis://127.0.0.1:6379/15）")
    })?;
    let database_url = std::env::var("DATABASE_URL")
        .map_err(|_| anyhow!("未找到 DATABASE_URL，请在仓库根目录 .env 中配置 PostgreSQL"))?;
    info!("测试 Redis: {}", redis_url);
    let redis_pool = build_redis_pool(&redis_url)?;
    flush_redis(&redis_pool).await?;
    let db = build_db_pool(&database_url).await?;

    // JWT 密钥：注入 config_manager，确保客户端签名与服务器校验一致
    setup_jwt_keys()?;

    // 自签名证书（写入系统临时目录，测试结束清理）
    let temp_dir = std::env::temp_dir().join(format!("quic_ext_test_{}", Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir).context("创建临时证书目录失败")?;
    let (cert_path, key_path) = make_cert_files(&temp_dir);

    let outcome = std::panic::AssertUnwindSafe(async {
        // 启动 ChatNode
        let addr = free_udp_addr();
        let mut config = ChatNodeConfig::new(addr);
        config.cert_path = cert_path;
        config.key_path = key_path;
        config.server_name = "localhost".to_string();
        config.node_address = addr.to_string();

        let core = make_core(db.clone(), redis_pool.clone());
        let mut node = ChatNode::new(config, core);
        node.init().await.context("ChatNode 初始化失败")?;
        node.start().await.context("ChatNode 启动失败")?;
        let node_addr = node.config().bind_address;
        info!("ChatNode 已启动，监听: {}", node_addr);

        // 测试用户：uuid + 有效 token
        let user_uuid = Uuid::new_v4().to_string();
        let access_token = generate_access_token(user_uuid.clone(), PC_PLATFORM.to_string())
            .context("生成 access_token 失败")?;
        let key = conn_key(PC_PLATFORM, &user_uuid);

        // 写入唯一测试消息，并将对应已读状态放入 Redis，验证上线阶段同步到数据库。
        let other_uuid = Uuid::new_v4();
        let message_uuid = Uuid::new_v4();
        let send_user: RbatisUuid =
            other_uuid.to_string().parse().context("解析发送者 UUID 失败")?;
        let recv_user: RbatisUuid =
            user_uuid.to_string().parse().context("解析接收者 UUID 失败")?;
        let message = ChatMessageRecord {
            id: None,
            nano_id: Some(message_uuid.to_string()),
            timestamp: Some(1_700_000_000_000),
            raw: b"integration-test".to_vec().into(),
            text_type: Some(1),
            send_user: send_user.clone(),
            recv_user: recv_user.clone(),
        };
        ChatMessageRecord::insert(&db, &message).await.context("写入测试聊天消息失败")?;
        let read_item = AddReadChatRecordDTO {
            nano_id: message.nano_id.clone(),
            timestamp: message.timestamp,
            send_user,
            recv_user,
            chat_type: Some(1),
        };
        let read_key = format!("{}{}", USER_READ_MSG, user_uuid).to_uppercase();
        {
            let mut conn = redis_pool.get().await.context("获取 Redis 连接失败")?;
            conn.set::<&str, String, ()>(&read_key, serde_json::to_string(&[read_item])?)
                .await
                .context("预置已读消息失败")?;
        }

        // ===== 1. 正常连接：握手 + 鉴权 + 注册 =====
        let head_len = head_size();
        let (client_endpoint, client_conn) = connect_client(node_addr).await?;
        let (_send, _recv) = send_first_msg(&client_conn, &user_uuid, &access_token, head_len)
            .await
            .context("发送初始化消息失败")?;
        info!("客户端已连接并发送 FirstQuicMsg");

        // 内存 connections 映射注册
        wait_connection_registered(&node, &key, Duration::from_secs(5)).await?;
        info!("连接已注册到内存映射: {}", key);

        // Redis 中也写入连接归属节点
        {
            let mut conn = redis_pool.get().await.context("获取 Redis 连接失败")?;
            let val: Option<String> = conn.get(&key).await.context("读取 Redis 连接 key 失败")?;
            assert_eq!(val.as_deref(), Some("0"), "Redis 应记录连接归属节点 0, 实际: {:?}", val);
        }
        info!("连接已注册到 Redis: {} -> 0", key);

        // ===== 2. 心跳：客户端发 PING，服务器回 PONG =====
        let ping = common::utils::text_msg::generate_text_msg(
            MSG_TYPE_PING,
            b"ping".to_vec(),
            SYSTEM.to_string(),
            user_uuid.clone(),
        )
        .context("构造 PING 消息失败")?;
        {
            let mut uni = client_conn.open_uni().await.context("打开 uni 流失败")?;
            uni.write_all(&ping).await.context("发送 PING 失败")?;
            uni.finish().await.context("结束 uni 流失败")?;
        }
        info!("已发送 PING，等待服务器回 PONG...");

        let pong_uni = tokio::time::timeout(Duration::from_secs(5), client_conn.accept_uni())
            .await
            .context("等待服务器 PONG uni 流超时")?
            .context("接收服务器 uni 流失败")?;
        let mut pong_buf = Vec::new();
        let mut chunk = [0u8; 4096];
        let mut recv = pong_uni;
        loop {
            match recv.read(&mut chunk).await {
                Ok(Some(n)) => pong_buf.extend_from_slice(&chunk[..n]),
                Ok(None) => break,
                Err(e) => return Err(anyhow!("读取 PONG 数据失败: {}", e)),
            }
        }
        let pong_len = pong_buf.len();
        let msgs =
            get_text_msg(&mut pong_buf, pong_len, Arc::new(Mutex::new(Vec::new())), head_len)
                .await
                .context("解析 PONG 消息失败")?;
        assert_eq!(msgs.len(), 1, "应解析出 1 条 PONG, 实际: {}", msgs.len());
        assert_eq!(msgs[0].text_type, MSG_TYPE_PING, "PONG 的 text_type 应为 MSG_TYPE_PING");
        assert_eq!(msgs[0].raw, PONG.as_bytes(), "PONG 的 raw 应为 {}", PONG);
        info!("心跳 PING/PONG 通过");

        // ===== 3a. 同一次登录重连(相同 jti):静默关闭旧连接,不发送 FORCE_LOGOUT =====
        let old_server_stable_id = node
            .connections()
            .get(&key)
            .map(|entry| entry.conn.stable_id())
            .ok_or_else(|| anyhow!("获取旧连接服务端 stable_id 失败"))?;
        let (second_endpoint, second_conn) = connect_client(node_addr).await?;
        let (_second_send, _second_recv) =
            send_first_msg(&second_conn, &user_uuid, &access_token, head_len).await?;
        let same_session_frame =
            recv_server_uni_or_closed(&client_conn, Duration::from_secs(5)).await?;
        assert!(
            same_session_frame.is_none(),
            "相同 token(相同 jti)重连属于同一次登录,应静默关闭旧连接而非强退"
        );
        wait_connection_changed(&node, &key, old_server_stable_id, Duration::from_secs(5)).await?;
        info!("同一次登录重连:旧连接已静默关闭并被新连接替换");

        let online_lock_key =
            format!("QUIC:ONLINE:LOCK:{}:{}", PC_PLATFORM, user_uuid).to_uppercase();
        let mut redis = redis_pool.get().await.context("获取 Redis 连接失败")?;
        let lock_value: Option<String> = redis.get(&online_lock_key).await?;
        assert!(lock_value.is_none(), "上线成功后应释放分布式锁");
        drop(redis);

        let read_uuid: RbatisUuid = user_uuid.to_string().parse().context("解析查询 UUID 失败")?;
        let read_rows = ChatMessageRecordRead::select_by_map(
            &db,
            rbs::value! { "nano_id": message_uuid.to_string() },
        )
        .await
        .context("查询已读消息失败")?;
        assert_eq!(read_rows.len(), 1, "上线阶段应将已读消息同步到数据库");
        assert_eq!(read_rows[0].recv_user, read_uuid);

        // 确保新 token 的 jti 严格晚于旧 token(避免同一毫秒内签发导致无法比较先后)
        tokio::time::sleep(Duration::from_millis(5)).await;

        // ===== 3b. 新登录(更新的 jti)接管:强退旧连接,FORCE_LOGOUT 携带发起会话 jti =====
        let access_token_newer = generate_access_token(user_uuid.clone(), PC_PLATFORM.to_string())
            .context("生成更新的 access_token 失败")?;
        let newer_jti = verify_token(&access_token_newer).context("解析新 token 失败")?.jti;
        let old_stable_before_takeover = node
            .connections()
            .get(&key)
            .map(|entry| entry.conn.stable_id())
            .ok_or_else(|| anyhow!("获取接管前连接 stable_id 失败"))?;
        let (third_endpoint, third_conn) = connect_client(node_addr).await?;
        let (_third_send, _third_recv) =
            send_first_msg(&third_conn, &user_uuid, &access_token_newer, head_len).await?;
        let kick_frame = recv_server_uni_or_closed(&second_conn, Duration::from_secs(5))
            .await?
            .ok_or_else(|| anyhow!("新登录接管应发送 FORCE_LOGOUT 强退旧连接"))?;
        let kick_len = kick_frame.len();
        let mut kick_buf = kick_frame;
        let kick_messages =
            get_text_msg(&mut kick_buf, kick_len, Arc::new(Mutex::new(Vec::new())), head_len)
                .await?;
        assert_eq!(kick_messages.len(), 1, "应解析出 1 条 FORCE_LOGOUT");
        assert_eq!(kick_messages[0].text_type, MSG_TYPE_FORCE_LOGOUT);
        assert_eq!(
            kick_session(&kick_messages[0].raw).as_deref(),
            Some(newer_jti.as_str()),
            "FORCE_LOGOUT 应携带发起接管的新会话 jti,便于客户端识别本机顶替而忽略"
        );
        wait_connection_changed(&node, &key, old_stable_before_takeover, Duration::from_secs(5))
            .await?;
        info!("新登录已强退旧连接并接管会话");

        // ===== 3c. 过期登录重连(更老的 jti):拒绝本次连接,保住更新的在线会话 =====
        let live_stable_id = node
            .connections()
            .get(&key)
            .map(|entry| entry.conn.stable_id())
            .ok_or_else(|| anyhow!("获取在线连接 stable_id 失败"))?;
        let live_session_id = node
            .connections()
            .get(&key)
            .map(|entry| entry.session_id.clone())
            .ok_or_else(|| anyhow!("获取在线会话 jti 失败"))?;
        let (stale_endpoint, stale_conn) = connect_client(node_addr).await?;
        // 复用最早的 access_token(其 jti 早于当前在线会话),模拟旧登录复活
        let (_stale_send, _stale_recv) =
            send_first_msg(&stale_conn, &user_uuid, &access_token, head_len).await?;
        let stale_frame = recv_server_uni_or_closed(&stale_conn, Duration::from_secs(5))
            .await?
            .ok_or_else(|| anyhow!("过期登录重连应收到 FORCE_LOGOUT 并被拒绝"))?;
        let stale_len = stale_frame.len();
        let mut stale_buf = stale_frame;
        let stale_messages =
            get_text_msg(&mut stale_buf, stale_len, Arc::new(Mutex::new(Vec::new())), head_len)
                .await?;
        assert_eq!(stale_messages.len(), 1, "过期登录应收到 1 条 FORCE_LOGOUT");
        assert_eq!(stale_messages[0].text_type, MSG_TYPE_FORCE_LOGOUT);
        assert_eq!(
            kick_session(&stale_messages[0].raw).as_deref(),
            Some(live_session_id.as_str()),
            "拒绝过期登录的 FORCE_LOGOUT 应携带在线会话 jti"
        );
        let still_live =
            node.connections().get(&key).map(|entry| entry.conn.stable_id()).unwrap_or(0);
        assert_eq!(still_live, live_stable_id, "过期登录不应顶掉更新的在线会话");
        let still_live_session =
            node.connections().get(&key).map(|entry| entry.session_id.clone()).unwrap_or_default();
        assert_eq!(still_live_session, live_session_id, "在线会话 jti 不应被过期登录改写");
        info!("过期登录重连已被拒绝,更新的在线会话保持不变");

        // ===== 4. 非法 token 被拒绝 =====
        let (bad_endpoint, bad_conn) = connect_client(node_addr).await?;
        let (_bad_send, mut bad_recv) =
            send_first_msg(&bad_conn, &user_uuid, "invalid-token", head_len)
                .await
                .context("发送非法 token 初始化消息失败")?;
        let read_result =
            tokio::time::timeout(Duration::from_secs(5), bad_recv.read(&mut [0u8; 16])).await;
        assert!(
            matches!(read_result, Ok(Ok(None))) || read_result.is_err(),
            "非法 token 应被服务器拒绝（流关闭/无数据），实际: {:?}",
            read_result
        );
        info!("非法 token 已被拒绝");

        // ===== 5. 客户端断开后连接清理 =====
        third_conn.close(0u32.into(), b"test done");
        wait_connection_removed(&node, &key, Duration::from_secs(5)).await?;
        info!("断开后连接已从内存映射清理");

        // 正常用户不产生多余注册（非法 token 连接不会写入 connections）
        assert!(
            node.connections().get(&conn_key(PC_PLATFORM, "nobody")).is_none(),
            "不应存在未注册用户"
        );

        bad_conn.close(0u32.into(), b"test done");
        bad_endpoint.wait_idle().await;
        drop(bad_endpoint);
        second_conn.close(0u32.into(), b"test done");
        second_endpoint.wait_idle().await;
        drop(second_endpoint);
        stale_conn.close(0u32.into(), b"test done");
        stale_endpoint.wait_idle().await;
        drop(stale_endpoint);
        third_endpoint.wait_idle().await;
        drop(third_endpoint);
        client_endpoint.wait_idle().await;
        drop(client_endpoint);

        // 关闭服务器
        node.stop().await.context("ChatNode 停止失败")?;
        info!("ChatNode 已停止");

        Ok::<(), anyhow::Error>(())
    })
    .catch_unwind()
    .await;

    // 清理：无论测试成功与否，清空测试 Redis 并删除临时证书
    if let Err(e) = flush_redis(&redis_pool).await {
        info!("清空测试 Redis 失败（不影响测试结果）: {}", e);
    }
    let _ = std::fs::remove_dir_all(&temp_dir);

    match outcome {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(e),
        Err(panic) => std::panic::resume_unwind(panic),
    }
}
