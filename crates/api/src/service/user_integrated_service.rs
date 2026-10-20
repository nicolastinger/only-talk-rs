use anyhow::anyhow;
use common::utils::server_count_sync::compute_preferred_index;
use http_service::http_service::notify_service::service::system_notification::{
    push_notification_via_quic, send_process_friend_msg, send_request_friend_msg,
};
use http_service::http_service::user_service::dto::friend_request_info_dto::FriendRequestInfoDTO;
use http_service::http_service::user_service::service::friend_service::{
    add_friend, process_friend,
};
use http_service::utils::http_response::CommonResponseNoDataRef;
use rbatis::RBatis;

pub async fn add_user_with_notify(
    rb: &RBatis,
    friend: FriendRequestInfoDTO,
) -> Result<String, anyhow::Error> {
    tracing::debug!(
        "开始添加好友流程: request_user={:?}, accept_user={:?}",
        friend.request_user,
        friend.accept_user
    );

    // 1. Add friend
    let friend_request = add_friend(rb, friend).await?;
    tracing::debug!(
        "好友请求已创建: uuid={:?}, request_user={:?}, accept_user={:?}",
        friend_request.uuid,
        friend_request.request_user,
        friend_request.accept_user
    );

    let target_uuid = friend_request.accept_user.ok_or(anyhow!("Please select a user"))?;
    let biz_id = friend_request
        .uuid
        .ok_or(anyhow!("Failed to add friend, request ID not found"))?
        .to_string();
    tracing::debug!("目标用户: target_uuid={}, biz_id={}", target_uuid, biz_id);

    // 2 发送系统通知 (落库)
    let quic_msg = send_request_friend_msg(
        rb,
        target_uuid,
        friend_request.request_message.clone().ok_or(anyhow!("Please provide a request reason"))?,
        Some(biz_id.clone()),
    )
    .await?;
    tracing::debug!(
        "系统通知已保存到数据库: quic_msg.user_id={:?}, request_message={:?}",
        quic_msg.user_id,
        friend_request.request_message
    );

    // 3. Forward notification via internal QUIC service (非致命: 推送失败不影响好友请求本身)
    if let Err(e) = push_notification_via_quic(&quic_msg).await {
        tracing::warn!("好友请求通知实时推送失败: {}", e);
    }

    Ok(CommonResponseNoDataRef::success_empty())
}

/// 处理好友请求
pub async fn process_friend_with_notify(
    rb: &RBatis,
    friend_request_info_dto: FriendRequestInfoDTO,
) -> Result<String, anyhow::Error> {
    // 1. Process friend request
    let friend_request = process_friend(rb, friend_request_info_dto).await?;
    let target_uuid = friend_request.request_user.ok_or(anyhow!("Please select a user"))?;
    let biz_id = friend_request
        .uuid
        .ok_or(anyhow!("Failed to add friend, request ID not found"))?
        .to_string();
    // 2. Send system notification (persist to DB)
    let accept_msg = friend_request
        .accept_message
        .filter(|m| !m.is_empty())
        .unwrap_or_else(|| "对方已处理您的好友申请".to_string());
    let quic_msg = send_process_friend_msg(rb, target_uuid, accept_msg, Some(biz_id)).await?;

    // 3. Forward notification via internal QUIC service (非致命)
    if let Err(e) = push_notification_via_quic(&quic_msg).await {
        tracing::warn!("好友处理通知实时推送失败: {}", e);
    }

    Ok(CommonResponseNoDataRef::success_empty())
}

/// 获取分配给当前用户的外网 QUIC 节点地址(哈希取模)
pub async fn get_quic_server_for_user(
    redis: &deadpool_redis::Pool,
    uuid: &str,
) -> Result<String, anyhow::Error> {
    use common::config_str::REDIS_EXTERNAL_QUIC_SERVERS;
    use common::utils::server_count_sync::get_server_count;
    use deadpool_redis::redis::AsyncCommands;
    use http_service::utils::http_response::CommonResponseRef;
    use serde::Serialize;
    use tracing::{info, warn};

    #[derive(Serialize)]
    struct QuicServerInfo {
        index: u32,
        address: String,
    }

    let sc = get_server_count();
    let index = compute_preferred_index(uuid);
    info!("QUIC 节点分配完成: server_count={} uuid={} index={}", sc, uuid, index);

    let mut conn = redis.get().await?;

    let key = format!("{}{}", REDIS_EXTERNAL_QUIC_SERVERS, index);
    let address: Option<String> = conn.get(&key).await?;
    let address = match address {
        Some(address) => address,
        None => {
            warn!(
                "外网 QUIC 节点不可用: redis key={} 不存在(节点未启动/未注册/注册已过期), server_count={} index={}",
                key, sc, index
            );
            return Err(anyhow!("无可用 QUIC 节点, 请稍后重试"));
        }
    };

    let info = QuicServerInfo { index, address };
    CommonResponseRef::success_json(&info).map_err(|e| anyhow!("Serialization failed: {}", e))
}

/// 获取 NAT UDP 端口配置(优先 Redis, 缺失时回退配置文件)
pub async fn get_nat_udp_ports(redis: &deadpool_redis::Pool) -> Result<String, anyhow::Error> {
    use common::config_str::NAT_UDP_PORTS;
    use common::read_global_config;
    use deadpool_redis::redis::AsyncCommands;
    use http_service::utils::http_response::CommonResponseRef;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Clone)]
    struct NatUdpPorts {
        v4_port_1: u16,
        v6_port_1: u16,
        v4_port_2: u16,
        v6_port_2: u16,
    }

    let mut conn = redis.get().await?;
    let raw: String = conn.get(NAT_UDP_PORTS).await.unwrap_or_default();
    let ports = if raw.trim().is_empty() {
        NatUdpPorts {
            v4_port_1: read_global_config!("nat_udp", "v4_port_1").parse()?,
            v6_port_1: read_global_config!("nat_udp", "v6_port_1").parse()?,
            v4_port_2: read_global_config!("nat_udp", "v4_port_2").parse()?,
            v6_port_2: read_global_config!("nat_udp", "v6_port_2").parse()?,
        }
    } else {
        serde_json::from_str(&raw)?
    };
    CommonResponseRef::success_json(&ports).map_err(|e| anyhow!("Serialization failed: {}", e))
}
