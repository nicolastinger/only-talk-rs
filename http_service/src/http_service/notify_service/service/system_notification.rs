use std::str::FromStr;
use anyhow::anyhow;
use rbatis::RBatis;
use rbs::value;
use uuid::Uuid;
use entity::models::chat_entity::chat_message_record::ChatMessageRecord;
use entity::models::notify_entity::system_notification::SystemNotification;
use entity::utils::time::get_now_time_stamp_as_millis;
use crate::utils::http_response::{CommonResponseNoDataRef, CommonResponseRef};

/// 新增好友请求通知
pub async fn send_request_friend_msg(
    rb: &RBatis,
    user_id: rbatis::rbdc::Uuid,
    msg: String,
    biz_id : Option<String>
) -> Result<SystemNotification, anyhow::Error> {
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
        biz_id,
        is_read: Some(false),
        level1: Some(1),
        level2: Some(1),
        level3: Some(1), // 1-新增
        level4: Some(0),

        unread_count: Some(1),
        priority: Some(1),
    };
    SystemNotification::insert(rb, &system_notification).await?;
    
    Ok(system_notification)
}

/// 新增好友处理通知
pub async fn send_process_friend_msg(
    rb: &RBatis,
    user_id: rbatis::rbdc::Uuid,
    msg: String,
    biz_id : Option<String>
) -> Result<SystemNotification, anyhow::Error> {
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
        biz_id,
        is_read: Some(false),
        level1: Some(1),
        level2: Some(1),
        level3: Some(2),  //2-处理
        level4: Some(0),

        unread_count: Some(1),
        priority: Some(1),
    };
    SystemNotification::insert(rb, &system_notification).await?;

    Ok(system_notification)
}

/// 获取用户未读的通知
pub async fn get_user_unread_notification(
    rb: &RBatis,
    user_id: Option<String>,                               
    is_read: Option<bool>
) -> Result<String, anyhow::Error> {
    let user_id = rbatis::rbdc::Uuid::from_str(user_id.ok_or(anyhow!("user_id is empty"))?.as_str())?;
    let system_notification = SystemNotification::select_all_by_uid(rb, &user_id, is_read).await?;
    Ok(CommonResponseRef::<Vec<SystemNotification>>::success_json(&system_notification)?)
}
