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

/// 初始化 SQL 连接池
pub async fn init_sql_pool(url: &str) -> Result<RBatis, anyhow::Error> {
    info!("正在连接数据库 - 地址: {}", url);
    let rb = RBatis::new();

    let mut opts = PgConnectOptions::new();
    opts.set_uri(url).map_err(|e| anyhow!("Failed to set database URI: {}", e))?;

    let conn_manager =
        ConnectionManager::new_arc(Arc::new(Box::new(PgDriver {})), Arc::new(Box::new(opts)));

    let pool = FastPool::new(conn_manager)
        .map_err(|e| anyhow!("Failed to create connection pool: {}", e))?;
    pool.set_timeout(Some(Duration::from_secs(2))).await;

    rb.pool
        .set(Box::new(pool))
        .map_err(|_e| Error::from("pool set fail!"))
        .map_err(|e| anyhow!("Failed to set connection pool: {}", e))?;

    info!("数据库连接池初始化成功");
    Ok(rb)
}
