use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use dashmap::DashMap;
use deadpool_redis::redis::AsyncCommands;
use tracing::{error, info, warn};

use common::config_str::{REDIS_INTERNAL_QUIC_SERVERS, REDIS_QUIC_SERVERS, REDIS_SPLIT};
use common::utils::internal_quic_client::send_internal_quic_msg;
use common::utils::internal_quic_msg::{InternalQuicRequest, InternalQuicResponse};
use common::utils::server_count_sync::get_server_count;
use common::REDIS_CLIENT;

use crate::models::quic_connection::{ConnectionType, QuicConnection};

/// 根据目标用户 UUID 计算首选节点序号
pub fn compute_preferred_index(uuid: &str) -> u32 {
    let sc = get_server_count();
    if sc <= 1 {
        return 0;
    }
    let mut hasher = DefaultHasher::new();
    uuid.hash(&mut hasher);
    (hasher.finish() as u32) % sc
}

/// 查 Redis 获取某序号节点的内网 QUIC 地址
async fn get_internal_addr_by_index(index: u32) -> Result<SocketAddr> {
    let redis = REDIS_CLIENT.read().await;
    let redis = redis.as_ref().ok_or_else(|| anyhow::anyhow!("Redis 未初始化"))?;
    let mut conn = redis.get().await?;
    let key = format!("{}{}", REDIS_INTERNAL_QUIC_SERVERS, index);
    let addr_str: String = conn.get(&key).await?;
    addr_str.parse().map_err(|e| anyhow::anyhow!("解析内网地址失败: {}", e))
}

/// Redis 兜底：查用户实际在哪个节点在线
async fn get_actual_node_index(uuid: &str, platform: &str) -> Result<Option<u32>> {
    let redis = REDIS_CLIENT.read().await;
    let redis = redis.as_ref().ok_or_else(|| anyhow::anyhow!("Redis 未初始化"))?;
    let mut conn = redis.get().await?;
    let key = format!(
        "{}:{}{}{}{}",
        platform,
        REDIS_QUIC_SERVERS,
        uuid,
        REDIS_SPLIT,
        ConnectionType::Text
    );
    let key = key.to_uppercase();
    let index_str: Option<String> = conn.get(&key).await?;
    Ok(index_str.and_then(|s| s.parse().ok()))
}

/// 尝试在本机投递消息（查本地 connections）
async fn try_deliver_local(
    request: &InternalQuicRequest,
    connections: &Arc<DashMap<String, QuicConnection>>,
) -> Result<Option<InternalQuicResponse>> {
    // 构建与 set_conn_info 一致的 connection key
    let connection_key = format!(
        "{}:{}{}{}{}",
        request.platform,
        REDIS_QUIC_SERVERS,
        request.target_user,
        REDIS_SPLIT,
        ConnectionType::Text
    );
    let connection_key = connection_key.to_uppercase();

    let conn = connections.get(&connection_key).map(|entry| entry.conn.clone());

    match conn {
        Some(conn) => {
            info!(
                "[路由] 本机投递成功 target={} platform={}",
                request.target_user, request.platform
            );
            let mut send = conn.open_uni().await?;
            send.write_all(request.payload.as_bytes()).await?;
            send.finish().await?;
            Ok(Some(InternalQuicResponse::ok()))
        }
        None => {
            info!(
                "[路由] 本机未找到 target={} platform={}",
                request.target_user, request.platform
            );
            Ok(None)
        }
    }
}

/// 转发到远程内网 QUIC
async fn forward_to_remote(
    addr: &SocketAddr,
    request: &InternalQuicRequest,
) -> Result<InternalQuicResponse> {
    info!(
        "[路由] 转发到远程节点 {} target={} preferred_index={} ttl={}",
        addr, request.target_user, request.preferred_index, request.ttl
    );
    send_internal_quic_msg(*addr, request.clone()).await
}

/// 内网 QUIC 两阶段路由核心
///
/// 阶段一：hash 取模路由
///   首选节点 == 本机 → 查本地 connections
///   首选节点 != 本机 → 转发到首选节点
///
/// 阶段二：Redis 兜底
///   查 PC/MOBILE:QUIC:SERVER:{uuid}:TEXT → 转发到实际节点
pub async fn route_request(
    request: &InternalQuicRequest,
    connections: &Arc<DashMap<String, QuicConnection>>,
    server_index: u32,
) -> Result<InternalQuicResponse> {
    let preferred_index = request.preferred_index;

    // ===== 阶段一：hash 取模路由 =====
    if preferred_index == server_index {
        // 首选节点 == 本机 → 尝试本机投递
        if let Some(resp) = try_deliver_local(request, connections).await? {
            return Ok(resp);
        }
        // 本机未找到 → 进入 Redis 兜底
    } else {
        // 首选节点 ≠ 本机 → 转发到首选节点
        if request.ttl > 0 {
            match get_internal_addr_by_index(preferred_index).await {
                Ok(target_addr) => {
                    let mut forward_req = request.clone();
                    forward_req.ttl -= 1;
                    let resp = forward_to_remote(&target_addr, &forward_req).await?;
                    if resp.delivered == Some(true) {
                        return Ok(resp);
                    }
                }
                Err(e) => {
                    warn!(
                        "[路由] 获取首选节点 {} 地址失败: {}，进入 Redis 兜底",
                        preferred_index, e
                    );
                }
            }
        }
    }

    // ===== 阶段二：Redis 兜底 =====
    if request.ttl == 0 {
        return Ok(InternalQuicResponse::user_offline());
    }

    let actual_index = get_actual_node_index(&request.target_user, &request.platform).await?;
    match actual_index {
        Some(idx) if idx == server_index => {
            // 实际就在本机，再试一次
            try_deliver_local(request, connections)
                .await
                .map(|r| r.unwrap_or_else(InternalQuicResponse::user_offline))
        }
        Some(idx) => {
            match get_internal_addr_by_index(idx).await {
                Ok(target_addr) => {
                    let mut forward_req = request.clone();
                    forward_req.ttl -= 1;
                    forward_to_remote(&target_addr, &forward_req).await
                }
                Err(e) => {
                    error!("[路由] Redis 兜底获取节点 {} 地址失败: {}", idx, e);
                    Ok(InternalQuicResponse::user_offline())
                }
            }
        }
        None => {
            info!(
                "[路由] 用户不在线 target={} platform={}",
                request.target_user, request.platform
            );
            Ok(InternalQuicResponse::user_offline())
        }
    }
}
