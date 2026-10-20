//! 集成测试共享设施: 建库/连接/删库与日志初始化。
//!
//! Rust 集成测试目录下每个 `*_test.rs` 是独立二进制, 通过 `mod common;` 复用本模块。
//! 每个测试二进制只会用到其中一部分函数, 因此统一放开 `dead_code`。
//!
//! 复用仓库根目录 `.env` 中的 `TEST_DATABASE_URL`(回退 `DATABASE_URL`)作为管理员连接,
//! 自动创建独立测试库并在其中执行 DDL, 不会触碰 `.env` 指向的开发库。
#![allow(dead_code)]

use std::time::Duration;

use anyhow::{Result, anyhow};
use rbatis::RBatis;
use rbatis::rbdc::db::ConnectOptions;
use rbatis::rbdc::pool::{ConnectionManager, Pool};
use rbdc_pg::PgDriver;
use rbdc_pg::options::PgConnectOptions;
use rbdc_pool_fast::FastPool;
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt};

/// ddl 集成测试默认库名(可用环境变量 `TEST_DATABASE_NAME` 覆盖)
pub const DEFAULT_TEST_DATABASE: &str = "only_talk_test";

/// session DAO 集成测试默认库名: 与 ddl 测试隔离, 两个测试二进制可并行运行
pub const DEFAULT_SESSION_TEST_DATABASE: &str = "only_talk_session_test";

/// 是否在测试结束后删除测试库(环境变量 `DROP_TEST_DATABASE`, 值为 1/true 时删除)
pub fn should_drop_test_database() -> bool {
    dotenvy::var("DROP_TEST_DATABASE")
        .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
        .unwrap_or(false)
}

/// 初始化 tracing, 日志输出到 stdout(默认 info 级别, 可用 RUST_LOG 覆盖)
pub fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = fmt().with_env_filter(filter).with_writer(std::io::stdout).try_init();
}

/// 读取测试数据库管理员连接(支持 `${VAR}` 占位符展开)。
/// 优先使用 `TEST_DATABASE_URL`(可指向独立的测试 PG 实例, 与开发库隔离);
/// 未设置时回退到 `DATABASE_URL`。
pub fn admin_database_url() -> Result<String> {
    dotenvy::dotenv().ok();
    let raw =
        std::env::var("TEST_DATABASE_URL").or_else(|_| std::env::var("DATABASE_URL")).map_err(
            |_| anyhow!("未找到 DATABASE_URL / TEST_DATABASE_URL，请确认仓库根目录存在 .env 文件"),
        )?;
    info!("测试数据库连接信息: {}", mask_database_url(&raw));
    Ok(expand_env_vars(&raw))
}

/// 脱敏打印数据库连接串, 避免把密码输出到日志(只保留 host:port 与库名)
pub fn mask_database_url(url: &str) -> String {
    match url.split_once('@') {
        Some((_, tail)) => format!("***@{}", tail),
        None => "***".to_string(),
    }
}

/// 测试库名, 默认 `only_talk_test`
pub fn test_database_name() -> String {
    test_database_name_or(DEFAULT_TEST_DATABASE)
}

/// 测试库名(指定默认值), 可用环境变量 `TEST_DATABASE_NAME` 覆盖
pub fn test_database_name_or(default: &str) -> String {
    dotenvy::var("TEST_DATABASE_NAME").unwrap_or_else(|_| default.to_string())
}

/// 将管理员连接 URL 的库名替换为测试库名, 其余连接信息保持不变
pub fn test_database_url(admin_url: &str, db_name: &str) -> Result<String> {
    validate_db_name(db_name)?;
    let at = admin_url.rfind('@').ok_or_else(|| anyhow!("无效的 DATABASE_URL: {}", admin_url))?;
    let (head, tail) = admin_url.split_at(at + 1);
    let slash =
        tail.find('/').ok_or_else(|| anyhow!("DATABASE_URL 缺少数据库名: {}", admin_url))?;
    Ok(format!("{}{}/{}", head, &tail[..slash], db_name))
}

/// 删除并重建测试库(若不存在则直接创建)。
///
/// 每次运行都必须从空库开始：分区表无法由普通表原地转换
/// (`CREATE TABLE IF NOT EXISTS ... PARTITION BY HASH` 遇到已存在的普通表会静默跳过，
/// 随后 `CREATE TABLE ... PARTITION OF` 因父表不是分区表而失败)。复用旧库会让 DDL 必失败。
pub async fn recreate_database(admin: &RBatis, db_name: &str) -> Result<()> {
    validate_db_name(db_name)?;
    drop_test_database(admin, db_name).await?;
    let create_sql = format!("CREATE DATABASE \"{}\"", db_name);
    admin
        .exec(&create_sql, vec![])
        .await
        .map_err(|e| anyhow!("创建测试库 {} 失败: {}", db_name, e))?;
    info!("已创建测试库 {}", db_name);
    Ok(())
}

/// 完成测试后执行删库(清理函数)。
///
/// 先终止测试库上的所有连接, 再执行 `DROP DATABASE`, 保证测试结束后不留残留、
/// 可重复运行。仅作用于传入的库名。
pub async fn drop_test_database(admin: &RBatis, db_name: &str) -> Result<()> {
    validate_db_name(db_name)?;
    let terminate_sql = format!(
        "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '{}' AND pid <> pg_backend_pid()",
        db_name
    );
    admin
        .query(&terminate_sql, vec![])
        .await
        .map_err(|e| anyhow!("终止测试库 {} 的连接失败: {}", db_name, e))?;
    let drop_sql = format!("DROP DATABASE IF EXISTS \"{}\"", db_name);
    admin
        .exec(&drop_sql, vec![])
        .await
        .map_err(|e| anyhow!("删除测试库 {} 失败: {}", db_name, e))?;
    info!("测试库 {} 已删除", db_name);
    Ok(())
}

/// 校验库名只能包含字母、数字、下划线
pub fn validate_db_name(db_name: &str) -> Result<()> {
    if db_name.is_empty() || !db_name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(anyhow!("非法测试库名: {}", db_name));
    }
    Ok(())
}

/// 展开字符串中的 `${KEY}` 占位符(来自 dotenvy 已加载的环境变量)
pub fn expand_env_vars(s: &str) -> String {
    let mut result = s.to_string();
    while let Some(start) = result.find("${") {
        if let Some(end_rel) = result[start + 2..].find('}') {
            let end = start + 2 + end_rel;
            let key = &result[start + 2..end];
            let value = dotenvy::var(key).unwrap_or_default();
            result.replace_range(start..=end, &value);
        } else {
            break;
        }
    }
    result
}

/// 构建 rbatis 连接池
pub async fn build_pool(url: &str) -> Result<RBatis> {
    let rb = RBatis::new();
    let mut opts = PgConnectOptions::new();
    opts.set_uri(url).map_err(|e| anyhow!("设置数据库 URI 失败: {}", e))?;
    let conn_manager = ConnectionManager::new_options(PgDriver {}, opts);
    let pool = FastPool::new(conn_manager).map_err(|e| anyhow!("创建连接池失败: {}", e))?;
    pool.set_timeout(Some(Duration::from_secs(2))).await;
    rb.pool
        .set(Box::new(pool))
        .map_err(|_e| rbatis::Error::from("pool set fail!"))
        .map_err(|e| anyhow!("设置连接池失败: {}", e))?;
    Ok(rb)
}
