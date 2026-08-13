//! 集成测试：将 `entity/ddl` 下全部建表脚本写入独立的测试数据库。
//!
//! 复用仓库根目录 `.env` 中的 `DATABASE_URL`（管理员连接）作为连接信息，
//! 自动创建测试库（默认 `only_talk_test`，可用 `TEST_DATABASE_NAME` 覆盖）
//! 并在其中执行全部 DDL，不会触碰 `.env` 指向的开发库。
//!
//! 运行方式：
//!   cargo test -p entity --test ddl_integration_test -- --ignored
//! 前提：本地 PostgreSQL 可用，且仓库根目录存在 `.env`。

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use rbatis::RBatis;
use rbatis::rbdc::db::ConnectOptions;
use rbatis::rbdc::pool::{ConnectionManager, Pool};
use rbdc_pg::PgDriver;
use rbdc_pg::options::PgConnectOptions;
use rbdc_pool_fast::FastPool;

/// 独立测试库名（可用环境变量 `TEST_DATABASE_NAME` 覆盖）
const DEFAULT_TEST_DATABASE: &str = "only_talk_test";

#[tokio::test]
#[ignore = "需要本地 PostgreSQL 与仓库根目录 .env"]
async fn apply_all_ddl_to_test_database() -> Result<()> {
    let admin_url = admin_database_url()?;
    let test_db_name = test_database_name();

    let admin = build_pool(&admin_url).await?;
    ensure_database(&admin, &test_db_name).await?;

    let test_url = test_database_url(&admin_url, &test_db_name)?;
    let test_rb = build_pool(&test_url).await?;

    entity::ddl::apply_all_ddl(&test_rb).await.context("应用 DDL 失败")?;

    for table in [
        "basic_user",
        "user_info",
        "friend_link",
        "friend_list",
        "chat_list_link",
        "chat_message_record",
        "file_upload_record",
        "group_info",
        "group_member",
        "system_notification",
    ] {
        assert!(table_exists(&test_rb, table).await?, "表 {} 未创建", table);
    }
    Ok(())
}

/// 读取 `.env` 中的 `DATABASE_URL`（支持 `${VAR}` 占位符展开）
fn admin_database_url() -> Result<String> {
    dotenvy::dotenv().ok();
    let raw = std::env::var("DATABASE_URL")
        .map_err(|_| anyhow!("未找到 DATABASE_URL，请确认仓库根目录存在 .env 文件"))?;
    Ok(expand_env_vars(&raw))
}

/// 测试库名，默认 `only_talk_test`
fn test_database_name() -> String {
    dotenvy::var("TEST_DATABASE_NAME").unwrap_or_else(|_| DEFAULT_TEST_DATABASE.to_string())
}

/// 将管理员连接 URL 的库名替换为测试库名，其余连接信息保持不变
fn test_database_url(admin_url: &str, db_name: &str) -> Result<String> {
    validate_db_name(db_name)?;
    let at = admin_url.rfind('@').ok_or_else(|| anyhow!("无效的 DATABASE_URL: {}", admin_url))?;
    let (head, tail) = admin_url.split_at(at + 1);
    let slash =
        tail.find('/').ok_or_else(|| anyhow!("DATABASE_URL 缺少数据库名: {}", admin_url))?;
    Ok(format!("{}{}/{}", head, &tail[..slash], db_name))
}

/// 若测试库不存在则创建（仅允许字母数字下划线，防注入）
async fn ensure_database(admin: &RBatis, db_name: &str) -> Result<()> {
    validate_db_name(db_name)?;
    let check_sql = format!("SELECT 1 FROM pg_database WHERE datname = '{}'", db_name);
    let result: rbs::Value = admin
        .query(&check_sql, vec![])
        .await
        .map_err(|e| anyhow!("查询 pg_database 失败: {}", e))?;
    let exists = result.as_array().map_or(0, |rows| rows.len()) > 0;
    if !exists {
        let create_sql = format!("CREATE DATABASE \"{}\"", db_name);
        admin
            .exec(&create_sql, vec![])
            .await
            .map_err(|e| anyhow!("创建测试库 {} 失败: {}", db_name, e))?;
    }
    Ok(())
}

/// 查询表是否存在
async fn table_exists(rb: &RBatis, table: &str) -> Result<bool> {
    let sql = format!(
        "SELECT 1 FROM information_schema.tables WHERE table_schema = 'public' AND table_name = '{}'",
        table
    );
    let result: rbs::Value =
        rb.query(&sql, vec![]).await.map_err(|e| anyhow!("查询表信息失败: {}", e))?;
    Ok(result.as_array().map_or(0, |rows| rows.len()) > 0)
}

/// 校验库名只能包含字母、数字、下划线
fn validate_db_name(db_name: &str) -> Result<()> {
    if db_name.is_empty() || !db_name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(anyhow!("非法测试库名: {}", db_name));
    }
    Ok(())
}

/// 展开字符串中的 `${KEY}` 占位符（来自 dotenvy 已加载的环境变量）
fn expand_env_vars(s: &str) -> String {
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
async fn build_pool(url: &str) -> Result<RBatis> {
    let rb = RBatis::new();
    let mut opts = PgConnectOptions::new();
    opts.set_uri(url).map_err(|e| anyhow!("设置数据库 URI 失败: {}", e))?;
    let conn_manager =
        ConnectionManager::new_arc(Arc::new(Box::new(PgDriver {})), Arc::new(Box::new(opts)));
    let pool = FastPool::new(conn_manager).map_err(|e| anyhow!("创建连接池失败: {}", e))?;
    pool.set_timeout(Some(Duration::from_secs(2))).await;
    rb.pool
        .set(Box::new(pool))
        .map_err(|_e| rbatis::Error::from("pool set fail!"))
        .map_err(|e| anyhow!("设置连接池失败: {}", e))?;
    Ok(rb)
}
