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

/// Initialize SQL connection pool (only effective on first call)
pub async fn init_sql_pool(url: &str) -> Result<RBatis, anyhow::Error> {
    if SQL_INIT_ONCE.get().is_some() {
        return RBATIS_DATABASE
            .read()
            .await
            .clone()
            .ok_or_else(|| anyhow!("Database not initialized"));
    }

    info!("connecting to database - address: {}", url);
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

    {
        let mut database = RBATIS_DATABASE
            .try_write()
            .map_err(|_| anyhow!("Failed to acquire database write lock"))?;
        if SQL_INIT_ONCE.set(()).is_ok() {
            *database = Some(rb.clone());
        }
    }

    info!("database connection pool initialized successfully");
    Ok(rb)
}

pub async fn get_sql_client() -> Result<RBatis, anyhow::Error> {
    let rb = RBATIS_DATABASE.read().await;
    rb.as_ref().ok_or(anyhow!("Failed to acquire connection")).cloned()
}
