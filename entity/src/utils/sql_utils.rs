use anyhow::anyhow;
use rbatis::RBatis;

use crate::RBATIS_DATABASE;

pub async fn get_sql_client() -> Result<RBatis, anyhow::Error> {
    let rb = RBATIS_DATABASE.read().await;
    rb.as_ref().ok_or(anyhow!("获取连接失败")).cloned()
}
