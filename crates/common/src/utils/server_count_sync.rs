use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, AtomicU32, Ordering};
use std::time::Duration;

use deadpool_redis::Pool;
use tracing::{error, info, warn};

use crate::config_str::{REDIS_EXTERNAL_QUIC_SERVERS, REDIS_INTERNAL_QUIC_SERVERS};
use crate::utils::mask::mask_addr;

/// 全局 server_count,无锁读取(热点路径)
pub static SERVER_COUNT: std::sync::LazyLock<Arc<AtomicU32>> =
    std::sync::LazyLock::new(|| Arc::new(AtomicU32::new(1)));

/// 外网 QUIC 节点注册 key TTL(秒)。比刷新周期(60s)大数倍,单次/短暂心跳丢失不至于掉节点
const EXTERNAL_NODE_KEY_TTL_SECS: u64 = 300;
/// 内网 QUIC 节点注册 key TTL(秒)
const INTERNAL_NODE_KEY_TTL_SECS: u64 = 7200;
/// 注册 key 刷新周期(秒)
const RENEW_INTERVAL_SECS: u64 = 60;
/// 单次刷新失败后的重试次数与间隔(秒)
const RENEW_RETRY_TIMES: u32 = 2;
const RENEW_RETRY_DELAY_SECS: u64 = 5;
/// 看门狗: 上次成功续期距今超过该毫秒数视为保活线程卡死/停跳
const RENEW_STALE_THRESHOLD_MS: i64 = 200_000;
/// 看门狗扫描周期(秒)
const WATCHDOG_INTERVAL_SECS: u64 = 30;
/// 看门狗持续告警去重间隔(毫秒)
const WATCHDOG_ALERT_GAP_MS: i64 = 300_000;

/// 最近一次"外网节点 key 续期成功"的时间戳(Unix 毫秒),供看门狗判断心跳是否停跳
static LAST_RENEW_OK_AT_MS: AtomicI64 = AtomicI64::new(0);
/// 看门狗上一次告警时间戳(Unix 毫秒),用于去重
static LAST_STALE_ALERT_AT_MS: AtomicI64 = AtomicI64::new(0);

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// 启动独立保活线程: 在独立 OS 线程 + 独立 tokio runtime + 独立 Redis 连接上,
/// 周期刷新本节点外网/内网注册 key 的 TTL。即使主 runtime 某一线程被卡死也不影响节点保活。
pub fn start_node_key_keeper(
    redis_url: String,
    server_index: u32,
    external_node_address: String,
    internal_node_address: String,
) {
    let builder = std::thread::Builder::new().name("node-key-keeper".to_string());
    let spawn_result = builder.spawn(move || {
        let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
            Ok(rt) => rt,
            Err(e) => {
                error!("节点 key 保活线程创建 runtime 失败: {}", e);
                return;
            }
        };
        runtime.block_on(run_node_key_keeper(
            redis_url,
            server_index,
            external_node_address,
            internal_node_address,
        ));
    });
    match spawn_result {
        Ok(_) => info!(
            "节点 key 保活线程已启动 (server_index={}, 外网TTL={}s, 内网TTL={}s, 每{}s刷新)",
            server_index,
            EXTERNAL_NODE_KEY_TTL_SECS,
            INTERNAL_NODE_KEY_TTL_SECS,
            RENEW_INTERVAL_SECS
        ),
        Err(e) => error!("节点 key 保活线程启动失败: {}", e),
    }
}

/// 启动独立看门狗线程: 纯 std::thread,不依赖任何 tokio runtime,
/// 通过检查"上次续期成功时间"判断保活线程是否停跳/卡死并告警。
pub fn start_node_key_watchdog() {
    let builder = std::thread::Builder::new().name("node-key-watchdog".to_string());
    let spawn_result = builder.spawn(|| loop {
        std::thread::sleep(Duration::from_secs(WATCHDOG_INTERVAL_SECS));
        let now = now_millis();
        let last_ok = LAST_RENEW_OK_AT_MS.load(Ordering::Relaxed);
        if last_ok <= 0 {
            continue;
        }
        let stale_ms = now - last_ok;
        if stale_ms > RENEW_STALE_THRESHOLD_MS {
            let last_alert = LAST_STALE_ALERT_AT_MS.load(Ordering::Relaxed);
            if now - last_alert > WATCHDOG_ALERT_GAP_MS {
                error!(
                    "节点 key 续期心跳停跳: 距上次成功续期已 {} 秒(>{}s), 保活线程疑似卡死, 节点注册 key 即将过期失联!",
                    stale_ms / 1000,
                    RENEW_STALE_THRESHOLD_MS / 1000
                );
                LAST_STALE_ALERT_AT_MS.store(now, Ordering::Relaxed);
            }
        }
    });
    match spawn_result {
        Ok(_) => info!(
            "节点 key 看门狗已启动 (停跳阈值={}s, 扫描周期={}s)",
            RENEW_STALE_THRESHOLD_MS / 1000,
            WATCHDOG_INTERVAL_SECS
        ),
        Err(e) => error!("节点 key 看门狗启动失败: {}", e),
    }
}

/// 保活线程主循环(运行在独立 runtime 上)
async fn run_node_key_keeper(
    redis_url: String,
    server_index: u32,
    external_node_address: String,
    internal_node_address: String,
) {
    let config = deadpool_redis::Config::from_url(&redis_url);
    let pool = match config.create_pool(Some(deadpool_redis::Runtime::Tokio1)) {
        Ok(pool) => pool,
        Err(e) => {
            error!("节点 key 保活线程创建 Redis 连接池失败: {}", e);
            return;
        }
    };

    let mut interval = tokio::time::interval(Duration::from_secs(RENEW_INTERVAL_SECS));
    loop {
        interval.tick().await;

        let external_key = format!("{}{}", REDIS_EXTERNAL_QUIC_SERVERS, server_index);
        let ext_ok = renew_key_with_retry(
            &pool,
            "外网",
            &external_key,
            &external_node_address,
            EXTERNAL_NODE_KEY_TTL_SECS,
        )
        .await;
        if ext_ok {
            LAST_RENEW_OK_AT_MS.store(now_millis(), Ordering::Relaxed);
        }

        let internal_key = format!("{}{}", REDIS_INTERNAL_QUIC_SERVERS, server_index);
        renew_key_with_retry(
            &pool,
            "内网",
            &internal_key,
            &internal_node_address,
            INTERNAL_NODE_KEY_TTL_SECS,
        )
        .await;
    }
}

/// 刷新单个注册 key 的 TTL,失败时短延时重试若干次,返回是否最终成功
async fn renew_key_with_retry(
    pool: &Pool,
    label: &str,
    key: &str,
    node_address: &str,
    ttl_secs: u64,
) -> bool {
    use deadpool_redis::redis::AsyncCommands;

    for attempt in 0..=RENEW_RETRY_TIMES {
        let mut conn = match pool.get().await {
            Ok(conn) => conn,
            Err(e) => {
                warn!(
                    "{} QUIC 节点 key 续期失败(获取连接): key={} err={} attempt={}/{}",
                    label, key, e, attempt, RENEW_RETRY_TIMES
                );
                if attempt < RENEW_RETRY_TIMES {
                    tokio::time::sleep(Duration::from_secs(RENEW_RETRY_DELAY_SECS)).await;
                }
                continue;
            }
        };
        match conn.set_ex::<&str, &str, ()>(key, node_address, ttl_secs).await {
            Ok(_) => {
                info!(
                    "{} QUIC 节点 key 续期成功: key={} value={} TTL={}s",
                    label,
                    key,
                    mask_addr(node_address),
                    ttl_secs
                );
                return true;
            }
            Err(e) => {
                warn!(
                    "{} QUIC 节点 key 续期失败: key={} err={} attempt={}/{}",
                    label, key, e, attempt, RENEW_RETRY_TIMES
                );
                if attempt < RENEW_RETRY_TIMES {
                    tokio::time::sleep(Duration::from_secs(RENEW_RETRY_DELAY_SECS)).await;
                }
            }
        }
    }
    false
}

/// 通过扫描 Redis 中实际在线的外网 QUIC 节点键来计算 server_count
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

/// 通过哈希取模计算首选节点索引
pub fn compute_preferred_index(uuid: &str) -> u32 {
    let sc = get_server_count();
    if sc <= 1 {
        return 0;
    }
    let mut hasher = DefaultHasher::new();
    uuid.hash(&mut hasher);
    (hasher.finish() as u32) % sc
}

/// 节点启动时注册外网 QUIC 节点到 Redis(短 TTL,由独立保活线程续期)
pub async fn register_external_node(
    pool: &Pool,
    server_index: u32,
    node_address: &str,
) -> Result<(), anyhow::Error> {
    use deadpool_redis::redis::AsyncCommands;
    let mut conn = pool.get().await?;
    let key = format!("{}{}", REDIS_EXTERNAL_QUIC_SERVERS, server_index);
    conn.set_ex::<&str, &str, ()>(&key, node_address, EXTERNAL_NODE_KEY_TTL_SECS).await?;
    info!(
        "外网 QUIC 节点已注册: key={} value={} TTL={}s",
        key,
        mask_addr(node_address),
        EXTERNAL_NODE_KEY_TTL_SECS
    );
    Ok(())
}

/// 启动 server_count 后台同步任务(每个节点启动时调用一次)。
/// 仅负责每 10s 扫描在线外网节点数并更新全局 SERVER_COUNT。
/// 注意: 节点 key 的保活续期已由独立保活线程 [start_node_key_keeper] 负责,这里不再参与。
pub fn start_server_count_sync(redis_pool: Pool) {
    tokio::spawn(async move {
        let mut count_interval = tokio::time::interval(Duration::from_secs(10));
        loop {
            count_interval.tick().await;
            match get_cluster_server_count(&redis_pool).await {
                Ok(count) if count > 0 => {
                    let old = SERVER_COUNT.swap(count, Ordering::Relaxed);
                    if old != count {
                        info!("server_count 已更新: {} → {}", old, count);
                    }
                }
                Ok(_) => {
                    warn!("server_count 异常: 数量为 0,忽略");
                }
                Err(e) => {
                    warn!("同步 server_count 失败: {}", e);
                }
            }
        }
    });
}
