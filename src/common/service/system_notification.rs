use std::str::FromStr;
use anyhow::anyhow;
use uuid::Uuid;
use crate::common::entity::system_notification::SystemNotification;
use crate::RBATIS_DATABASE;
use crate::utils::time::get_now_time_stamp_as_millis;

/// 发送好友请求
pub async fn send_request_friend_msg(
    user_id: rbatis::rbdc::Uuid,
    msg: String,
) -> Result<(), anyhow::Error> {
    // 1 插入数据库
    let now = get_now_time_stamp_as_millis()?;
    let uuid = Uuid::now_v7().to_string();
    let system_notification = SystemNotification {
        id: Some(rbatis::rbdc::Uuid::from_str(uuid.as_str())?),
        title: Some("好友通知".to_string()),
        content: Some(msg),
        created_at: Some(now),
        content_type: Some(0),
        user_id: Some(user_id),
        is_read: Some(false),
        level1: Some(1),
        level2: Some(1),
        level3: Some(1),
        level4: Some(0),

        unread_count: Some(1),
    };
    let rb = RBATIS_DATABASE.read().await;
    let rb = rb.as_ref().ok_or(anyhow!("获取连接失败"))?;
    SystemNotification::insert(rb, &system_notification).await?;

    // 2 转发通知到具体用户
    Ok(())
}
