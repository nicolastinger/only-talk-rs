use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use deadpool_redis::Pool;
use tracing::{info, warn};

use crate::config_str::REDIS_EXTERNAL_QUIC_SERVERS;

/// 全局 server_count，lock-free 读取（热点路径）
pub static SERVER_COUNT: std::sync::LazyLock<Arc<AtomicU32>> =
    std::sync::LazyLock::new(|| Arc::new(AtomicU32::new(1)));

/// 启动后台同步任务（每个节点启动时调用一次）
/// 同时负责定期刷新本节点外网 QUIC 注册 key 的 TTL
pub fn start_server_count_sync(redis_pool: Pool, server_index: u32, node_address: String) {
    tokio::spawn(async move {
        let mut count_interval = tokio::time::interval(Duration::from_secs(10));
        let mut refresh_interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            tokio::select! {
                _ = count_interval.tick() => {
                    match get_cluster_server_count(&redis_pool).await {
                        Ok(count) if count > 0 => {
                            let old =  SERVER_COUNT.swap(count, Ordering::Relaxed);
                            if old != count {
                                info!("server_count updated: {} → {}", old, count);
                            }
                        }
                        Ok(_) => {
                            warn!("server_count anomaly: count is 0, ignoring");
                        }
                        Err(e) => {
                            warn!("failed to sync server_count: {}", e);
                        }
                    }
                }
                _ = refresh_interval.tick() => {
                    refresh_external_node_key(&redis_pool, server_index, &node_address).await;
                }
            }
        }
    });
}

/// 刷新本节点外网 QUIC 注册 key 的 TTL
async fn refresh_external_node_key(pool: &Pool, server_index: u32, node_address: &str) {
    use deadpool_redis::redis::AsyncCommands;
    if let Ok(mut conn) = pool.get().await {
        let key = format!("{}{}", REDIS_EXTERNAL_QUIC_SERVERS, server_index);
        let _: Result<(), _> = conn
            .set_ex::<&str, &str, ()>(&key, node_address, 120)
            .await;
    }
}

/// 通过扫描 Redis 中实际在线的外网 QUIC 节点 key 来计算 server_count
async fn get_cluster_server_count(pool: &Pool) -> Result<u32, anyhow::Error> {
    use deadpool_redis::redis::AsyncCommands;
    let mut conn = pool.get().await?;
    let pattern = format!("{}*", REDIS_EXTERNAL_QUIC_SERVERS);
    let keys: Vec<String> = conn.keys(&pattern).await?;
    let count = keys.len() as u32;
    Ok(count)
}

/// 读取当前 server_count
pub fn get_server_count() -> u32 {
    SERVER_COUNT.load(Ordering::Relaxed)
}

/// hash 取模计算首选节点序号
pub fn compute_preferred_index(uuid: &str) -> u32 {
    let sc = get_server_count();
    if sc <= 1 {
        return 0;
    }
    let mut hasher = DefaultHasher::new();
    uuid.hash(&mut hasher);
    (hasher.finish() as u32) % sc
}

/// 节点启动时注册外网 QUIC 节点到 Redis（短 TTL，由后台任务续期）
pub async fn register_external_node(pool: &Pool, server_index: u32, node_address: &str) -> Result<(), anyhow::Error> {
    use deadpool_redis::redis::AsyncCommands;
    let mut conn = pool.get().await?;
    let key = format!("{}{}", REDIS_EXTERNAL_QUIC_SERVERS, server_index);
    conn.set_ex::<&str, &str, ()>(&key, node_address, 120).await?;
    info!("external QUIC node registered: key={} value={} TTL=120s", key, node_address);
    Ok(())
}
