use std::str::FromStr;
use anyhow::anyhow;
use rbatis::RBatis;
use rbatis::rbdc::Uuid;
use crate::http_service::user_service::dto::friend_request_info_dto::FriendRequestInfoDTO;
use crate::http_service::user_service::service::friend_service::add_friend;
use crate::http_service::notify_service::service::system_notification::send_request_friend_msg;
use crate::utils::http_response::CommonResponseNoDataRef;

pub async fn add_user_with_notify(rb: &RBatis, friend: FriendRequestInfoDTO) -> Result<String, anyhow::Error> {
    let target_id = friend.accept_user.clone().ok_or(anyhow!("请选择一个用户"))?;
    let target_id = Uuid::from_str(target_id.as_str())?;
    let msg = friend.request_message.clone().unwrap_or("".to_string());
    // 1 添加好友
    add_friend(rb, friend).await?;

    // 2 发送通知
    send_request_friend_msg(rb, target_id, msg).await?;
    Ok(CommonResponseNoDataRef::success_empty())
}