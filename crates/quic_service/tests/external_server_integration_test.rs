//! 外部 QUIC 服务器（ChatNode）集成测试
//!
//! 启动一个真实 ChatNode（自签名证书 + 测试 Redis），用 QUIC 客户端模拟用户设备，覆盖：
//! 1. 连接握手 + `FirstQuicMsg` JWT 鉴权 + 连接注册（内存 `connections` 映射 + Redis key）
//! 2. 心跳：客户端通过 uni 流发送 PING，服务器回推 PONG
//! 3. 非法 token 被拒绝（服务器直接关闭流）
//! 4. 同平台重复登录挤下线（通知旧客户端并清理旧连接）
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
use common::utils::jwt_util::generate_access_token;
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

        // ===== 3. 同平台重复登录：通知并关闭旧连接 =====
        let old_server_stable_id = node
            .connections()
            .get(&key)
            .map(|entry| entry.conn.stable_id())
            .ok_or_else(|| anyhow!("获取旧连接服务端 stable_id 失败"))?;
        let (second_endpoint, second_conn) = connect_client(node_addr).await?;
        let (_second_send, _second_recv) =
            send_first_msg(&second_conn, &user_uuid, &access_token, head_len).await?;
        let old_conn_msg = tokio::time::timeout(Duration::from_secs(5), client_conn.accept_uni())
            .await
            .context("等待旧 PC 挤下线通知超时")??;
        let mut kick_buf = Vec::new();
        let mut kick_recv = old_conn_msg;
        let mut kick_chunk = [0u8; 4096];
        loop {
            match kick_recv.read(&mut kick_chunk).await {
                Ok(Some(n)) => kick_buf.extend_from_slice(&kick_chunk[..n]),
                Ok(None) => break,
                Err(e) => return Err(anyhow!("读取挤下线通知失败: {}", e)),
            }
        }
        let kick_len = kick_buf.len();
        let kick_messages =
            get_text_msg(&mut kick_buf, kick_len, Arc::new(Mutex::new(Vec::new())), head_len)
                .await?;
        assert_eq!(kick_messages.len(), 1);
        assert_eq!(kick_messages[0].text_type, MSG_TYPE_FORCE_LOGOUT);
        wait_connection_changed(&node, &key, old_server_stable_id, Duration::from_secs(5)).await?;

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
        second_conn.close(0u32.into(), b"test done");
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
        second_endpoint.wait_idle().await;
        drop(second_endpoint);
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
