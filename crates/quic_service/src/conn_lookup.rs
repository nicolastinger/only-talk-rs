//! 连接表(DashMap)的安全读取与单向流发送入口。
//!
//! 约束: 凡是"读出 quinn::Connection 之后还要异步发送/投递"的场合,都必须经由本模块
//! 的函数获取连接。这些函数在返回前已释放 DashMap 的 guard(读锁),
//! 从构造上杜绝"guard 跨 .await 持有"导致的死锁:
//! 持读锁的任务在 open_uni/write 上挂起时,同 key(同 shard)的断线清理想拿写锁
//! 会 park 所在线程,在单线程 runtime 上双方永久互等,冻结整个 QUIC 接收端。
//!
//! 注: 若调用方确实需要 guard(读 update_time/session_id、remove、比较 stable_id),
//! 请保持作用域最小,并在任何 .await 之前显式 drop。

use std::time::Duration;

use anyhow::Result;
use common::config_str::{MOBILE_PLATFORM, PC_PLATFORM, REDIS_QUIC_SERVERS, REDIS_SPLIT};
use quinn::Connection;
use tokio::time::timeout;

use crate::ConnectionsMap;
use crate::models::quic_connection::ConnectionType;

/// 单向流发送整体超时: 对端断网/半死连接时写可能长时间无 ACK,
/// 超时即返回错误,避免无限占用 worker 线程与 uni 流配额。
const UNI_SEND_TIMEOUT: Duration = Duration::from_secs(10);

fn build_key(platform: &str, uuid: &str, connection_type: &ConnectionType) -> String {
    format!("{}:{}{}{}{}", platform, REDIS_QUIC_SERVERS, uuid, REDIS_SPLIT, connection_type)
        .to_uppercase()
}

/// 按完整连接 key 读取连接,返回所有权(guard 在返回前已释放)。
pub fn get_conn_by_key(connections: &ConnectionsMap, key: &str) -> Option<Connection> {
    connections.get(key).map(|entry| entry.conn.clone())
}

/// 按 uuid + 平台 + 连接类型读取连接,返回所有权(guard 在返回前已释放)。
pub fn get_conn(
    connections: &ConnectionsMap,
    uuid: &str,
    platform: &str,
    connection_type: &ConnectionType,
) -> Option<Connection> {
    get_conn_by_key(connections, &build_key(platform, uuid, connection_type))
}

/// 按 uuid 依次尝试 PC / MOBILE 两个平台,返回任一在线的连接(群聊/系统消息推送用)。
pub fn get_conn_any_platform(connections: &ConnectionsMap, uuid: &str) -> Option<Connection> {
    for platform in [PC_PLATFORM, MOBILE_PLATFORM] {
        if let Some(conn) = get_conn(connections, uuid, platform, &ConnectionType::Text) {
            return Some(conn);
        }
    }
    None
}

/// 向一条已持有的连接发送完整 uni 帧(open_uni + write_all + finish),带超时。
pub async fn send_uni_frame(conn: &Connection, data: &[u8]) -> Result<()> {
    timeout(UNI_SEND_TIMEOUT, async {
        let mut send = conn.open_uni().await?;
        send.write_all(data).await?;
        send.finish().await?;
        Ok::<(), anyhow::Error>(())
    })
    .await
    .map_err(|_| anyhow::anyhow!("uni 流发送超时(>{:?})", UNI_SEND_TIMEOUT))?
}
