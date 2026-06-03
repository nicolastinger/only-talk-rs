use std::net::SocketAddr;

use anyhow::anyhow;
use common::utils::message_types::NOTIFY_TYPE_MSG;
use common::utils::internal_quic_msg::{InternalQuicRequest, RequestSource};
use common::utils::server_count_sync::compute_preferred_index;
use common::read_global_config;
use http_service::http_service::notify_service::service::system_notification::{
    send_process_friend_msg, send_request_friend_msg,
};
use http_service::http_service::user_service::dto::friend_request_info_dto::FriendRequestInfoDTO;
use http_service::http_service::user_service::service::friend_service::{
    add_friend, process_friend,
};
use http_service::utils::http_response::CommonResponseNoDataRef;
use rbatis::RBatis;
use common::utils::internal_quic_client::send_internal_quic_msg;

pub async fn add_user_with_notify(
    rb: &RBatis,
    friend: FriendRequestInfoDTO,
) -> Result<String, anyhow::Error> {
    tracing::debug!("starting add friend flow: request_user={:?}, accept_user={:?}",
        friend.request_user, friend.accept_user);
    
    // 1. Add friend
    let friend_request = add_friend(rb, friend).await?;
    tracing::debug!("friend request created: uuid={:?}, request_user={:?}, accept_user={:?}", 
        friend_request.uuid, friend_request.request_user, friend_request.accept_user);

    let target_uuid = friend_request.accept_user.ok_or(anyhow!("Please select a user"))?;
    let biz_id = friend_request.uuid.ok_or(anyhow!("Failed to add friend, request ID not found"))?.to_string();
    tracing::debug!("target user: target_uuid={}, biz_id={}", target_uuid, biz_id);

    // 2 发送系统通知 (落库)
    let quic_msg = send_request_friend_msg(
        rb,
        target_uuid,
        friend_request.request_message.clone().ok_or(anyhow!("Please provide a request reason"))?,
        Some(biz_id.clone()),
    )
    .await?;
    tracing::debug!("system notification saved to db: quic_msg.user_id={:?}, request_message={:?}", 
        quic_msg.user_id, friend_request.request_message);
    
    let json_str: String = serde_json::to_string(&quic_msg)?;
    let target_id_str = quic_msg.user_id.ok_or(anyhow!("Notification missing target user ID"))?.to_string();
    tracing::debug!("notification JSON serialized: target_id={}, payload_length={}", target_id_str, json_str.len());

    // 3. Forward notification via internal QUIC service
    let addr_str = read_global_config!("internal_quic_server", "address");
    let server_addr: SocketAddr = addr_str.parse()?;
    let preferred_index = compute_preferred_index(&target_id_str);
    tracing::debug!("QUIC service config: address={}, server_addr={}, preferred_index={}", 
        addr_str, server_addr, preferred_index);
    
    let request = InternalQuicRequest {
        msg_type: NOTIFY_TYPE_MSG,
        payload: json_str.clone().into_bytes(),
        target_user: target_id_str.clone(),
        preferred_index,
        platform: common::config_str::PC_PLATFORM.to_string(),
        source: RequestSource::HttpApi,
        ttl: 3,
    };
    tracing::debug!("preparing to send QUIC message: msg_type={}, target_user={}, platform={}, source={:?}, ttl={}", 
        request.msg_type, request.target_user, request.platform, request.source, request.ttl);
    
    send_internal_quic_msg(server_addr, request).await?;
    tracing::debug!("QUIC message sent successfully: target_user={}", target_id_str);

    Ok(CommonResponseNoDataRef::success_empty())
}

/// Process friend request
pub async fn process_friend_with_notify(
    rb: &RBatis,
    friend_request_info_dto: FriendRequestInfoDTO,
) -> Result<String, anyhow::Error> {
    // 1. Process friend request
    let friend_request = process_friend(rb, friend_request_info_dto).await?;
    let target_uuid = friend_request.request_user.ok_or(anyhow!("Please select a user"))?;
    let biz_id = friend_request.uuid.ok_or(anyhow!("Failed to add friend, request ID not found"))?.to_string();
    // 2. Send system notification (persist to DB)
    let accept_msg = friend_request
        .accept_message
        .filter(|m| !m.is_empty())
        .unwrap_or_else(|| "对方已处理您的好友申请".to_string());
    let quic_msg = send_process_friend_msg(
        rb,
        target_uuid,
        accept_msg,
        Some(biz_id),
    )
    .await?;
    let json_str: String = serde_json::to_string(&quic_msg)?;
    let target_id_str = quic_msg.user_id.ok_or(anyhow!("Notification missing target user ID"))?.to_string();

    // 3. Forward notification via internal QUIC service
    let addr_str = read_global_config!("internal_quic_server", "address");
    let server_addr: SocketAddr = addr_str.parse()?;
    let preferred_index = compute_preferred_index(&target_id_str);
    let request = InternalQuicRequest {
        msg_type: NOTIFY_TYPE_MSG,
        payload: json_str.into_bytes(),
        target_user: target_id_str,
        preferred_index,
        platform: common::config_str::PC_PLATFORM.to_string(),
        source: RequestSource::HttpApi,
        ttl: 3,
    };
    send_internal_quic_msg(server_addr, request).await?;

    Ok(CommonResponseNoDataRef::success_empty())
}

/// Get external QUIC node address assigned to current user (hash modulo)
pub async fn get_quic_server_for_user(uuid: &str) -> Result<String, anyhow::Error> {
    use deadpool_redis::redis::AsyncCommands;
    use common::config_str::REDIS_EXTERNAL_QUIC_SERVERS;
    use common::utils::server_count_sync::get_server_count;
    use http_service::utils::http_response::CommonResponseRef;
    use serde::Serialize;
    use tracing::info;

    #[derive(Serialize)]
    struct QuicServerInfo {
        index: u32,
        address: String,
    }

    let sc = get_server_count();
    let index = compute_preferred_index(uuid);
    info!("QUIC node assigned: server_count={} uuid={} index={}", sc, uuid, index);

    let redis = common::REDIS_CLIENT.read().await;
    let redis = redis.as_ref().ok_or(anyhow!("Redis not initialized"))?;
    let mut conn = redis.get().await?;

    let key = format!("{}{}", REDIS_EXTERNAL_QUIC_SERVERS, index);
    let address: String = conn.get(&key).await?;

    let info = QuicServerInfo { index, address };
    CommonResponseRef::success_json(&info).map_err(|e| anyhow!("Serialization failed: {}", e))
}
