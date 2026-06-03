use std::sync::Arc;
use std::time::Duration;

use anyhow::anyhow;
use rbatis::rbdc::db::ConnectOptions;
use rbatis::{Error, RBatis, rbdc};
use rbdc::pool::{ConnectionManager, Pool};
use rbdc_pg::PgDriver;
use rbdc_pg::options::PgConnectOptions;
use rbdc_pool_fast::FastPool;
use tracing::info;

use crate::{RBATIS_DATABASE, SQL_INIT_ONCE};

/// 初始化 SQL 连接池（仅首次调用生效）
pub async fn init_sql_pool(url: &str) -> Result<RBatis, anyhow::Error> {
    if SQL_INIT_ONCE.get().is_some() {
        return RBATIS_DATABASE
            .read()
            .await
            .clone()
            .ok_or_else(|| anyhow!("数据库未初始化"));
    }

    info!("connecting to database - address: {}", url);
    let rb = RBatis::new();

    let mut opts = PgConnectOptions::new();
    opts.set_uri(url).map_err(|e| anyhow!("设置数据库URI失败: {}", e))?;

    let conn_manager =
        ConnectionManager::new_arc(Arc::new(Box::new(PgDriver {})), Arc::new(Box::new(opts)));

    let pool = FastPool::new(conn_manager).map_err(|e| anyhow!("创建连接池失败: {}", e))?;
    pool.set_timeout(Some(Duration::from_secs(2))).await;

    rb.pool
        .set(Box::new(pool))
        .map_err(|_e| Error::from("pool set fail!"))
        .map_err(|e| anyhow!("设置连接池失败: {}", e))?;

    {
        let mut database = RBATIS_DATABASE
            .try_write()
            .map_err(|_| anyhow!("获取数据库写锁失败"))?;
        if SQL_INIT_ONCE.set(()).is_ok() {
            *database = Some(rb.clone());
        }
    }

    info!("database connection pool initialized successfully");
    Ok(rb)
}

pub async fn get_sql_client() -> Result<RBatis, anyhow::Error> {
    let rb = RBATIS_DATABASE.read().await;
    rb.as_ref().ok_or(anyhow!("获取连接失败")).cloned()
}
