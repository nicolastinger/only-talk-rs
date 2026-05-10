use std::net::SocketAddr;

use anyhow::anyhow;
use entity::utils::message_types::NOTIFY_TYPE_MSG;
use entity::models::internal_quic_msg::InternalQuicRequest;
use entity::read_global_config;
use http_service::http_service::notify_service::service::system_notification::{
    send_process_friend_msg, send_request_friend_msg,
};
use http_service::http_service::user_service::dto::friend_request_info_dto::FriendRequestInfoDTO;
use http_service::http_service::user_service::service::friend_service::{
    add_friend, process_friend,
};
use http_service::utils::http_response::CommonResponseNoDataRef;
use rbatis::RBatis;
use entity::utils::internal_quic_client::send_internal_quic_msg;

pub async fn add_user_with_notify(
    rb: &RBatis,
    friend: FriendRequestInfoDTO,
) -> Result<String, anyhow::Error> {
    // 1 添加好友
    let friend_request = add_friend(rb, friend).await?;

    let target_uuid = friend_request.accept_user.ok_or(anyhow!("请选择一个用户"))?;
    let biz_id = friend_request.uuid.ok_or(anyhow!("添加好友失败，找不到请求id"))?.to_string();

    // 2 发送系统通知 (落库)
    let quic_msg = send_request_friend_msg(
        rb,
        target_uuid,
        friend_request.request_message.ok_or(anyhow!("请填写申请理由"))?,
        Some(biz_id),
    )
    .await?;
    let json_str: String = serde_json::to_string(&quic_msg)?;
    let target_id = quic_msg.user_id.ok_or(anyhow!("请填写申请理由"))?.to_string();

    // 3 通过内网QUIC服务转发通知
    let addr_str = read_global_config!("internal_quic_server", "address");
    let server_addr: SocketAddr = addr_str.parse()?;
    let request = InternalQuicRequest {
        msg_type: NOTIFY_TYPE_MSG,
        payload: json_str,
        target_user: target_id,
    };
    send_internal_quic_msg(server_addr, request).await?;

    Ok(CommonResponseNoDataRef::success_empty())
}

/// 处理好友申请
pub async fn process_friend_with_notify(
    rb: &RBatis,
    friend_request_info_dto: FriendRequestInfoDTO,
) -> Result<String, anyhow::Error> {
    // 1、处理好友申请
    let friend_request = process_friend(rb, friend_request_info_dto).await?;
    let target_uuid = friend_request.request_user.ok_or(anyhow!("请选择一个用户"))?;
    // 2、插入系统通知
    let biz_id = friend_request.uuid.ok_or(anyhow!("添加好友失败，找不到请求id"))?.to_string();
    let quic_msg = send_process_friend_msg(
        rb,
        target_uuid,
        friend_request.accept_message.ok_or(anyhow!("请填写申请理由"))?,
        Some(biz_id),
    )
    .await?;
    let json_str: String = serde_json::to_string(&quic_msg)?;
    let target_id = quic_msg.user_id.ok_or(anyhow!("请填写申请理由"))?.to_string();

    // 3、通过内网QUIC服务转发通知
    let addr_str = read_global_config!("internal_quic_server", "address");
    let server_addr: SocketAddr = addr_str.parse()?;
    let request = InternalQuicRequest {
        msg_type: NOTIFY_TYPE_MSG,
        payload: json_str,
        target_user: target_id,
    };
    send_internal_quic_msg(server_addr, request).await?;

    Ok(CommonResponseNoDataRef::success_empty())
}

/// 获取可用的外网 QUIC 服务器列表（从 Redis 读取）
pub async fn get_quic_server_list() -> Result<String, anyhow::Error> {
    use deadpool_redis::redis::AsyncCommands;
    use entity::config_str::REDIS_EXTERNAL_QUIC_SERVERS;
    use http_service::utils::http_response::CommonResponseRef;
    use serde::Serialize;

    let redis = entity::REDIS_CLIENT.read().await;
    let redis = redis.as_ref().ok_or(anyhow!("Redis 未初始化"))?;
    let mut conn = redis.get().await?;

    let pattern = format!("{}*", REDIS_EXTERNAL_QUIC_SERVERS);
    let keys: Vec<String> = conn.keys(&pattern).await?;

    #[derive(Serialize)]
    struct QuicServerInfo {
        name: String,
        address: String,
    }

    let mut servers = Vec::new();
    for key in &keys {
        let addr: String = conn.get(key).await?;
        let name = key.replace(REDIS_EXTERNAL_QUIC_SERVERS, "");
        servers.push(QuicServerInfo {
            name,
            address: addr,
        });
    }

    CommonResponseRef::success_json(&servers)
        .map_err(|e| anyhow!("序列化失败: {}", e))
}
