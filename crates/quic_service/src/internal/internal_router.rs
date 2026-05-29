use std::hash::Hasher;
use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use dashmap::DashMap;
use deadpool_redis::redis::AsyncCommands;
use nanoid::nanoid;
use tracing::{error, info, warn};

use common::config_str::{REDIS_INTERNAL_QUIC_SERVERS, REDIS_QUIC_SERVERS, REDIS_SPLIT};
use common::utils::group_msg::{
    InternalGroupBroadcast, InternalGroupBroadcastResponse,
};
use common::utils::internal_quic_client::send_internal_quic_msg;
use common::utils::internal_quic_msg::{InternalQuicRequest, InternalQuicResponse};
use common::utils::message_types::NOTIFY_TYPE_MSG;
use common::REDIS_CLIENT;

use crate::models::quic_connection::{ConnectionType, QuicConnection};
use crate::msg_service::group_msg_service::process_group_broadcast;
use crate::msg_service::text_msg_service::generate_text_msg_with_id;

async fn get_internal_addr_by_index(index: u32) -> Result<SocketAddr> {
    let redis = REDIS_CLIENT.read().await;
    let redis = redis.as_ref().ok_or_else(|| anyhow::anyhow!("Redis 未初始化"))?;
    let mut conn = redis.get().await?;
    let key = format!("{}{}", REDIS_INTERNAL_QUIC_SERVERS, index);
    let addr_str: String = conn.get(&key).await?;
    addr_str.parse().map_err(|e| anyhow::anyhow!("解析内网地址失败: {}", e))
}

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

async fn try_deliver_local(
    request: &InternalQuicRequest,
    connections: &Arc<DashMap<String, QuicConnection>>,
) -> Result<Option<InternalQuicResponse>> {
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
                "[路由] 本机投递成功 target={} platform={} msg_type={}",
                request.target_user, request.platform, request.msg_type
            );
            let mut send = conn.open_uni().await?;
            
            if request.msg_type == NOTIFY_TYPE_MSG {
                let nano_id = nanoid!();
                let raw = request.payload.as_bytes().to_vec();
                let msg_bytes = generate_text_msg_with_id(
                    nano_id,
                    NOTIFY_TYPE_MSG,
                    raw,
                    request.target_user.clone(),
                    "system".to_string(),
                )?;
                info!(
                    "[路由] 通知消息已包装为TextQuicMsg格式 target={} payload_len={}",
                    request.target_user, request.payload.len()
                );
                send.write_all(&msg_bytes).await?;
            } else {
                send.write_all(request.payload.as_bytes()).await?;
            }
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

pub async fn route_request(
    request: &InternalQuicRequest,
    connections: &Arc<DashMap<String, QuicConnection>>,
    server_index: u32,
) -> Result<InternalQuicResponse> {
    let preferred_index = request.preferred_index;

    if preferred_index == server_index {
        if let Some(resp) = try_deliver_local(request, connections).await? {
            return Ok(resp);
        }
    } else {
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

    if request.ttl == 0 {
        return Ok(InternalQuicResponse::user_offline());
    }

    let actual_index = get_actual_node_index(&request.target_user, &request.platform).await?;
    match actual_index {
        Some(idx) if idx == server_index => {
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

pub async fn route_internal_request(
    request: &[u8],
    connections: &Arc<DashMap<String, QuicConnection>>,
    server_index: u32,
) -> Result<Vec<u8>> {
    if let Ok(broadcast) = bincode::deserialize::<InternalGroupBroadcast>(request) {
        info!(
            "[内网QUIC] 收到群聊广播 group_uuid={} sender={} members_count={}",
            broadcast.group_uuid,
            broadcast.sender,
            broadcast.all_members.len()
        );
        process_group_broadcast(&broadcast, connections).await?;
        return Ok(bincode::serialize(&InternalGroupBroadcastResponse::ok())?);
    }

    if let Ok(msg) = bincode::deserialize::<InternalQuicRequest>(request) {
        let resp = route_request(&msg, connections, server_index).await?;
        return Ok(bincode::serialize(&resp)?);
    }

    Err(anyhow::anyhow!("未知的内网 QUIC 请求类型"))
}
