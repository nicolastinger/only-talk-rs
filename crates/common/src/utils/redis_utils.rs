use anyhow::anyhow;
use deadpool_redis::Connection;
use deadpool_redis::redis::cmd;
use deadpool_redis::{Config as RedisConfig, Pool, Runtime};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::{REDIS_CLIENT, REDIS_INIT_ONCE};

/// Initialize Redis connection pool (only effective on first call)
pub fn init_redis(url: &str) -> Result<Pool, anyhow::Error> {
    if REDIS_INIT_ONCE.get().is_some() {
        return REDIS_CLIENT
            .try_read()
            .map_err(|_| anyhow!("Failed to acquire Redis read lock"))?
            .clone()
            .ok_or_else(|| anyhow!("Redis not initialized"));
    }

    info!("connecting to Redis - address: {}", url);
    let config = RedisConfig::from_url(url);
    let pool = config
        .create_pool(Some(Runtime::Tokio1))
        .map_err(|e| anyhow!("Failed to create Redis connection pool: {}", e))?;

    {
        let mut guard =
            REDIS_CLIENT.try_write().map_err(|_| anyhow!("Failed to acquire Redis write lock"))?;
        if REDIS_INIT_ONCE.set(()).is_ok() {
            *guard = Some(pool.clone());
        }
    }

    info!("Redis connection pool initialized successfully");
    Ok(pool)
}

pub async fn verify_redis(pool: &Pool) {
    match pool.get().await {
        Ok(mut conn) => {
            let result: Result<String, _> =
                deadpool_redis::redis::cmd("PING").query_async(&mut conn).await;
            match result {
                Ok(ref s) if s == "PONG" => {
                    info!("Redis connected (PING: {})", s);
                }
                Ok(s) => {
                    warn!("Redis PING returned anomaly: {}", s);
                }
                Err(e) => {
                    error!("Redis connection failed: {}", e);
                }
            }
        }
        Err(e) => {
            error!("Redis connection acquisition failed: {}", e);
        }
    }
}

pub async fn get_redis_conn() -> Result<Connection, anyhow::Error> {
    let redis_client = REDIS_CLIENT.read().await;
    let redis_conn = redis_client.as_ref().ok_or(anyhow!("Redis client error"))?;
    let conn = redis_conn.get().await?;
    Ok(conn)
}

/// Redis connection fallback: returns None when unavailable, no error
pub async fn try_get_redis_conn() -> Option<Connection> {
    let redis = REDIS_CLIENT.read().await;
    redis.as_ref()?.get().await.ok()
}

/// Redis distributed lock acquire
pub async fn acquire_lock(
    conn: &mut Connection,
    key: &str,
    ttl_sec: u64,
    content: String,
) -> Result<Option<String>, anyhow::Error> {
    let lock_id = Uuid::new_v4().to_string(); // Generate unique identifier
    let lock_id = format!("{}_{}", lock_id, content);
    let result: Option<()> = cmd("SET")
        .arg(key)
        .arg(&lock_id)
        .arg("NX") // Mutual exclusion: only set when key does not exist
        .arg("EX") // Expiry time unit: seconds
        .arg(ttl_sec)
        .query_async(conn)
        .await?;

    Ok(if result.is_some() {
        Some(lock_id) // Return lock identifier for subsequent release
    } else {
        None
    })
}

/// Redis distributed lock release
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

    Ok(deleted == 1) // Whether the lock was successfully released
}
