use std::str::FromStr;
use rbatis::RBatis;
use uuid::Uuid;
use entity::models::notify_entity::system_notification::SystemNotification;
use entity::utils::time::get_now_time_stamp_as_millis;

/// 发送好友请求通知
pub async fn send_request_friend_msg(
    rb: &RBatis,
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
    SystemNotification::insert(rb, &system_notification).await?;
    
    Ok(())
}
