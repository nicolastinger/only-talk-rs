use anyhow::anyhow;
use deadpool_redis::Connection;
use deadpool_redis::redis::cmd;
use deadpool_redis::{Config as RedisConfig, Pool, Runtime};
use tracing::info;
use uuid::Uuid;

use crate::{REDIS_CLIENT, REDIS_INIT_ONCE};

/// 初始化 Redis 连接池（仅首次调用生效）
pub fn init_redis(url: &str) -> Result<Pool, anyhow::Error> {
    if REDIS_INIT_ONCE.get().is_some() {
        return REDIS_CLIENT
            .try_read()
            .map_err(|_| anyhow!("获取Redis读锁失败"))?
            .clone()
            .ok_or_else(|| anyhow!("Redis未初始化"));
    }

    info!("正在连接 Redis - 地址: {}", url);
    let config = RedisConfig::from_url(url);
    let pool = config
        .create_pool(Some(Runtime::Tokio1))
        .map_err(|e| anyhow!("创建Redis连接池失败: {}", e))?;

    {
        let mut guard = REDIS_CLIENT
            .try_write()
            .map_err(|_| anyhow!("获取Redis写锁失败"))?;
        if REDIS_INIT_ONCE.set(()).is_ok() {
            *guard = Some(pool.clone());
        }
    }

    info!("Redis 连接池初始化成功");
    Ok(pool)
}

pub async fn get_redis_conn() -> Result<Connection, anyhow::Error> {
    let redis_client = REDIS_CLIENT.read().await;
    let redis_conn = redis_client.as_ref().ok_or(anyhow!("redis客户端错误"))?;
    let conn = redis_conn.get().await?;
    Ok(conn)
}

/// redis分布式锁加锁
pub async fn acquire_lock(
    conn: &mut Connection,
    key: &str,
    ttl_sec: u64,
    content: String,
) -> Result<Option<String>, anyhow::Error> {
    let lock_id = Uuid::new_v4().to_string(); // 生成唯一标识
    let lock_id = format!("{}_{}", lock_id, content);
    let result: Option<()> = cmd("SET")
        .arg(key)
        .arg(&lock_id)
        .arg("NX") // 互斥性：仅当 key 不存在时设置
        .arg("EX") // 过期时间单位秒
        .arg(ttl_sec)
        .query_async(conn)
        .await?;

    Ok(if result.is_some() {
        Some(lock_id) // 返回锁标识，用于后续释放
    } else {
        None
    })
}

/// redis分布式锁释放
pub async fn release_lock(
    conn: &mut Connection,
    key: &str,
    lock_id: &str,
) -> Result<bool, anyhow::Error> {
    let script = r#"
        if redis.call("GET", KEYS[1]) == ARGV[1] then
            return redis.call("DEL", KEYS[1])
        else
            return 0
        end
    "#;
    let deleted: i32 =
        cmd("EVAL").arg(script).arg(1).arg(key).arg(lock_id).query_async(conn).await?;

    Ok(deleted == 1) // 是否成功释放
}
