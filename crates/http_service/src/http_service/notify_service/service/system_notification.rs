use std::net::SocketAddr;
use std::str::FromStr;

use anyhow::anyhow;
use common::config_str::NOTIFY_RETENTION_DAYS;
use common::models::notify_entity::system_notification::{SystemNotification, mark_read_by_ids};
use common::read_global_config;
use common::utils::internal_quic_client::send_internal_quic_msg;
use common::utils::internal_quic_msg::{InternalQuicRequest, RequestSource};
use common::utils::message_types::NOTIFY_TYPE_MSG;
use common::utils::server_count_sync::compute_preferred_index;
use common::utils::time::get_now_time_stamp_as_millis;
use rbatis::RBatis;
use uuid::Uuid;

use crate::utils::http_response::{CommonResponseNoDataRef, CommonResponseRef};

/// 通过内部 QUIC 服务实时推送系统通知给目标用户
pub async fn push_notification_via_quic(
    notification: &SystemNotification,
) -> Result<(), anyhow::Error> {
    let target_id = notification
        .user_id
        .as_ref()
        .map(|u| u.to_string())
        .ok_or_else(|| anyhow!("Notification missing target user ID"))?;
    let json_str = serde_json::to_string(notification)?;

    // 包装为 TextQuicMsg 二进制(与其他消息路径保持一致)
    let payload = common::utils::text_msg::generate_text_msg(
        NOTIFY_TYPE_MSG,
        json_str.into_bytes(),
        target_id.clone(),
        common::config_str::SYSTEM.to_string(),
    )?;

    let addr_str = read_global_config!("internal_quic_server", "address");
    let server_addr: SocketAddr = addr_str.parse()?;
    let preferred_index = compute_preferred_index(&target_id);

    let request = InternalQuicRequest {
        msg_type: NOTIFY_TYPE_MSG,
        payload,
        target_user: target_id,
        preferred_index,
        platform: common::config_str::PC_PLATFORM.to_string(),
        source: RequestSource::HttpApi,
        ttl: 3,
        close_after_delivery: false,
        incoming_session: String::new(),
        send_force_logout: false,
    };

    send_internal_quic_msg(server_addr, request).await.map(|_| ())
}

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

/// 新增广场心动通知
pub async fn send_plaza_like_msg(
    rb: &RBatis,
    user_id: rbatis::rbdc::Uuid,
    msg: String,
    biz_id: Option<String>,
) -> Result<SystemNotification, anyhow::Error> {
    let now = get_now_time_stamp_as_millis()?;
    let uuid = Uuid::now_v7().to_string();
    let system_notification = SystemNotification {
        id: Some(rbatis::rbdc::Uuid::from_str(uuid.as_str())?),
        title: Some("交友广场通知".to_string()),
        content: Some(msg),
        created_at: Some(now),
        content_type: Some(0),
        user_id: Some(user_id),
        biz_id,
        is_read: Some(false),
        level1: Some(1),
        level2: Some(4), // 交友广场通知
        level3: Some(1), // 心动
        level4: Some(0),
        unread_count: Some(1),
        priority: Some(1),
    };
    SystemNotification::insert(rb, &system_notification).await?;
    Ok(system_notification)
}

/// 新增广场互相心动(匹配)通知
pub async fn send_plaza_match_msg(
    rb: &RBatis,
    user_id: rbatis::rbdc::Uuid,
    msg: String,
    biz_id: Option<String>,
) -> Result<SystemNotification, anyhow::Error> {
    let now = get_now_time_stamp_as_millis()?;
    let uuid = Uuid::now_v7().to_string();
    let system_notification = SystemNotification {
        id: Some(rbatis::rbdc::Uuid::from_str(uuid.as_str())?),
        title: Some("交友广场通知".to_string()),
        content: Some(msg),
        created_at: Some(now),
        content_type: Some(0),
        user_id: Some(user_id),
        biz_id,
        is_read: Some(false),
        level1: Some(1),
        level2: Some(4), // 交友广场通知
        level3: Some(2), // 互相心动(匹配)
        level4: Some(0),
        unread_count: Some(1),
        priority: Some(1),
    };
    SystemNotification::insert(rb, &system_notification).await?;
    Ok(system_notification)
}

/// 新增动态点赞通知
pub async fn send_moment_like_msg(
    rb: &RBatis,
    user_id: rbatis::rbdc::Uuid,
    msg: String,
    biz_id: Option<String>,
) -> Result<SystemNotification, anyhow::Error> {
    let now = get_now_time_stamp_as_millis()?;
    let uuid = Uuid::now_v7().to_string();
    let system_notification = SystemNotification {
        id: Some(rbatis::rbdc::Uuid::from_str(uuid.as_str())?),
        title: Some("动态通知".to_string()),
        content: Some(msg),
        created_at: Some(now),
        content_type: Some(0),
        user_id: Some(user_id),
        biz_id,
        is_read: Some(false),
        level1: Some(1),
        level2: Some(5), // 动态广场通知
        level3: Some(1), // 点赞
        level4: Some(0),
        unread_count: Some(1),
        priority: Some(1),
    };
    SystemNotification::insert(rb, &system_notification).await?;
    Ok(system_notification)
}

/// 新增动态评论通知
pub async fn send_moment_comment_msg(
    rb: &RBatis,
    user_id: rbatis::rbdc::Uuid,
    msg: String,
    biz_id: Option<String>,
) -> Result<SystemNotification, anyhow::Error> {
    let now = get_now_time_stamp_as_millis()?;
    let uuid = Uuid::now_v7().to_string();
    let system_notification = SystemNotification {
        id: Some(rbatis::rbdc::Uuid::from_str(uuid.as_str())?),
        title: Some("动态通知".to_string()),
        content: Some(msg),
        created_at: Some(now),
        content_type: Some(0),
        user_id: Some(user_id),
        biz_id,
        is_read: Some(false),
        level1: Some(1),
        level2: Some(5), // 动态广场通知
        level3: Some(2), // 评论
        level4: Some(0),
        unread_count: Some(1),
        priority: Some(1),
    };
    SystemNotification::insert(rb, &system_notification).await?;
    Ok(system_notification)
}

/// 获取用户未读的通知（仅最近 NOTIFY_RETENTION_DAYS 天内）
pub async fn get_user_unread_notification(
    rb: &RBatis,
    user_id: Option<String>,
    is_read: Option<bool>,
) -> Result<String, anyhow::Error> {
    let user_id =
        rbatis::rbdc::Uuid::from_str(user_id.ok_or(anyhow!("user_id is empty"))?.as_str())?;
    let now = get_now_time_stamp_as_millis()?;
    let since = now - NOTIFY_RETENTION_DAYS * 24 * 3600 * 1000;
    let is_read = is_read.unwrap_or(false);
    let system_notification =
        SystemNotification::select_unread_in_window(rb, &user_id, is_read, since).await?;
    Ok(CommonResponseRef::<Vec<SystemNotification>>::success_json(&system_notification)?)
}

/// 批量标记已读（幂等）
pub async fn mark_notifications_read(
    rb: &RBatis,
    user_id: Option<String>,
    ids: Vec<String>,
) -> Result<String, anyhow::Error> {
    let user_id =
        rbatis::rbdc::Uuid::from_str(user_id.ok_or(anyhow!("user_id is empty"))?.as_str())?;
    mark_read_by_ids(rb, &user_id, &ids).await?;
    Ok(CommonResponseNoDataRef::success_empty())
}
