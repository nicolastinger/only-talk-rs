use anyhow::anyhow;
use entity::utils::message_types::NOTIFY_TYPE_MSG;
use http_service::http_service::notify_service::service::system_notification::send_request_friend_msg;
use http_service::http_service::user_service::dto::friend_request_info_dto::FriendRequestInfoDTO;
use http_service::http_service::user_service::service::friend_service::add_friend;
use http_service::utils::http_response::CommonResponseNoDataRef;
use quic_service::msg_service::send_msg::send_quic_system_msg;
use rbatis::RBatis;
use rbatis::rbdc::Uuid;
use std::str::FromStr;

pub async fn add_user_with_notify(
    rb: &RBatis,
    friend: FriendRequestInfoDTO,
) -> Result<String, anyhow::Error> {
    let target_id = friend
        .accept_user
        .clone()
        .ok_or(anyhow!("请选择一个用户"))?;
    let target_uuid = Uuid::from_str(target_id.as_str())?;
    // 1 添加好友
    let friend_request = add_friend(rb, friend).await?;

    let biz_id = friend_request
        .uuid
        .ok_or(anyhow!("添加好友失败，找不到请求id"))?
        .to_string();
    
    // 2 发送通知
    let quic_msg = send_request_friend_msg(rb, target_uuid, friend_request.request_message.ok_or(anyhow!("请填写申请理由"))?, Some(biz_id)).await?;

    // 3 发送quic通知
    send_quic_system_msg(target_id, NOTIFY_TYPE_MSG, quic_msg.content.ok_or(anyhow!("请填写申请理由"))?).await?;
    Ok(CommonResponseNoDataRef::success_empty())
}
