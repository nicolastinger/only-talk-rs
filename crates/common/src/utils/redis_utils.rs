use anyhow::anyhow;
use deadpool_redis::redis::cmd;
use deadpool_redis::{Config as RedisConfig, Connection, Pool, Runtime};
use tracing::{error, info, warn};
use uuid::Uuid;

/// 初始化 Redis 连接池
pub fn init_redis(url: &str) -> Result<Pool, anyhow::Error> {
    info!("正在连接 Redis - 地址: {}", url);
    let config = RedisConfig::from_url(url);
    let pool = config
        .create_pool(Some(Runtime::Tokio1))
        .map_err(|e| anyhow!("Failed to create Redis connection pool: {}", e))?;
    info!("Redis 连接池初始化成功");
    Ok(pool)
}

pub async fn verify_redis(pool: &Pool) {
    match pool.get().await {
        Ok(mut conn) => {
            let result: Result<String, _> =
                deadpool_redis::redis::cmd("PING").query_async(&mut conn).await;
            match result {
                Ok(ref s) if s == "PONG" => {
                    info!("Redis 连接成功 (PING: {})", s);
                }
                Ok(s) => {
                    warn!("Redis PING 返回异常: {}", s);
                }
                Err(e) => {
                    error!("Redis 连接失败: {}", e);
                }
            }
        }
        Err(e) => {
            error!("获取 Redis 连接失败: {}", e);
        }
    }
}

/// 获取 Redis 分布式锁
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
        .arg("NX") // 互斥: 仅当键不存在时设置
        .arg("EX") // 过期时间单位: 秒
        .arg(ttl_sec)
        .query_async(conn)
        .await?;

    Ok(if result.is_some() {
        Some(lock_id) // 返回锁标识,供后续释放使用
    } else {
        None
    })
}

/// 释放 Redis 分布式锁
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

    Ok(deleted == 1) // 锁是否成功释放
}
