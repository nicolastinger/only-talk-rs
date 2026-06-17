use std::str::FromStr;

use anyhow::anyhow;
use common::models::notify_entity::system_notification::SystemNotification;
use common::utils::time::get_now_time_stamp_as_millis;
use rbatis::RBatis;
use uuid::Uuid;

use crate::utils::http_response::CommonResponseRef;

/// 新增好友请求通知
pub async fn send_request_friend_msg(
    rb: &RBatis,
    user_id: rbatis::rbdc::Uuid,
    msg: String,
    biz_id: Option<String>,
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
    biz_id: Option<String>,
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
        level3: Some(2), //2-处理
        level4: Some(0),

        unread_count: Some(1),
        priority: Some(1),
    };
    SystemNotification::insert(rb, &system_notification).await?;

    Ok(system_notification)
}

/// 新增群邀请通知
pub async fn send_group_invite_msg(
    rb: &RBatis,
    user_id: rbatis::rbdc::Uuid,
    msg: String,
    biz_id: Option<String>,
) -> Result<SystemNotification, anyhow::Error> {
    let now = get_now_time_stamp_as_millis()?;
    let uuid = Uuid::now_v7().to_string();
    let system_notification = SystemNotification {
        id: Some(rbatis::rbdc::Uuid::from_str(uuid.as_str())?),
        title: Some("群邀请通知".to_string()),
        content: Some(msg),
        created_at: Some(now),
        content_type: Some(0),
        user_id: Some(user_id),
        biz_id,
        is_read: Some(false),
        level1: Some(1),
        level2: Some(3), // 群聊通知
        level3: Some(1), // 群邀请
        level4: Some(0),
        unread_count: Some(1),
        priority: Some(1),
    };
    SystemNotification::insert(rb, &system_notification).await?;
    Ok(system_notification)
}

/// 群邀请处理结果通知（通知群主/管理员邀请已被处理）
pub async fn send_group_invite_result_msg(
    rb: &RBatis,
    user_id: rbatis::rbdc::Uuid,
    msg: String,
    biz_id: Option<String>,
) -> Result<SystemNotification, anyhow::Error> {
    let now = get_now_time_stamp_as_millis()?;
    let uuid = Uuid::now_v7().to_string();
    let system_notification = SystemNotification {
        id: Some(rbatis::rbdc::Uuid::from_str(uuid.as_str())?),
        title: Some("群邀请结果".to_string()),
        content: Some(msg),
        created_at: Some(now),
        content_type: Some(0),
        user_id: Some(user_id),
        biz_id,
        is_read: Some(false),
        level1: Some(1),
        level2: Some(3), // 群聊通知
        level3: Some(4), // 群邀请结果
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
    is_read: Option<bool>,
) -> Result<String, anyhow::Error> {
    let user_id =
        rbatis::rbdc::Uuid::from_str(user_id.ok_or(anyhow!("user_id is empty"))?.as_str())?;
    let system_notification = SystemNotification::select_all_by_uid(rb, &user_id, is_read).await?;
    Ok(CommonResponseRef::<Vec<SystemNotification>>::success_json(&system_notification)?)
}
