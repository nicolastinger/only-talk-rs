use std::str::FromStr;
use anyhow::anyhow;
use rbatis::RBatis;
use rbatis::rbdc::Uuid;
use entity::utils::message_types::MSG_TYPE_SYSTEM_START;
use http_service::http_service::user_service::dto::friend_request_info_dto::FriendRequestInfoDTO;
use http_service::http_service::user_service::service::friend_service::add_friend;
use http_service::http_service::notify_service::service::system_notification::send_request_friend_msg;
use http_service::utils::http_response::CommonResponseNoDataRef;
use quic_service::msg_service::send_msg::send_quic_system_msg;

pub async fn add_user_with_notify(rb: &RBatis, friend: FriendRequestInfoDTO) -> Result<String, anyhow::Error> {
    let target_id = friend.accept_user.clone().ok_or(anyhow!("请选择一个用户"))?;
    let target_uuid = Uuid::from_str(target_id.as_str())?;
    let msg = friend.request_message.clone().unwrap_or("".to_string());

    let quic_msg = msg.clone();
    // 1 添加好友
    add_friend(rb, friend).await?;

    // 2 发送通知
    send_request_friend_msg(rb, target_uuid, msg).await?;

    // 3 发送quic通知
    send_quic_system_msg(target_id, MSG_TYPE_SYSTEM_START, quic_msg).await?;
    Ok(CommonResponseNoDataRef::success_empty())
}