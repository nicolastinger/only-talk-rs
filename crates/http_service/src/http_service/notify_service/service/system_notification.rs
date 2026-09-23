use std::net::SocketAddr;
use std::str::FromStr;

use anyhow::anyhow;
use common::config_str::{
    MOBILE_PLATFORM, NOTIFY_LEVEL1_SYSTEM, NOTIFY_LEVEL2_FRIEND, NOTIFY_LEVEL2_GROUP,
    NOTIFY_LEVEL2_MOMENT, NOTIFY_LEVEL2_PLAZA, NOTIFY_LEVEL3_FRIEND_PROCESS,
    NOTIFY_LEVEL3_FRIEND_REQUEST, NOTIFY_LEVEL3_GROUP_INVITE, NOTIFY_LEVEL3_GROUP_INVITE_RESULT,
    NOTIFY_LEVEL3_MOMENT_COMMENT, NOTIFY_LEVEL3_MOMENT_LIKE, NOTIFY_LEVEL3_PLAZA_LIKE,
    NOTIFY_LEVEL3_PLAZA_MATCH, NOTIFY_LIST_DEFAULT_SIZE, NOTIFY_LIST_MAX_SIZE,
    NOTIFY_RETENTION_DAYS, PC_PLATFORM, SYSTEM,
};
use common::models::notify_entity::system_notification::{
    NotificationPageQuery, SystemNotification, mark_all_read, mark_read_by_ids,
};
use common::read_global_config;
use common::utils::internal_quic_client::send_internal_quic_msg;
use common::utils::internal_quic_msg::{InternalQuicRequest, RequestSource};
use common::utils::message_types::NOTIFY_TYPE_MSG;
use common::utils::server_count_sync::compute_preferred_index;
use common::utils::time::get_now_time_stamp_as_millis;
use rbatis::RBatis;
use rbatis::rbdc::Uuid;
use uuid::Uuid as UuidNow;

use crate::http_service::notify_service::dto::{
    MarkReadDTO, NotificationCursor, NotificationLevelCountVO, NotificationListDTO,
    NotificationListVO, NotificationUnreadSummaryVO, SystemNotificationVO,
};
use crate::utils::http_response::{CommonResponseNoDataRef, CommonResponseRef};

/// 构造通知的语义化入参(收敛 8 个 send_* 的重复字段)
struct NotificationSpec {
    title: &'static str,
    content: String,
    user_id: rbatis::rbdc::Uuid,
    biz_id: Option<String>,
    level1: i32,
    level2: i32,
    level3: i32,
    level4: i32,
    priority: i32,
    /// 是否同源聚合(如多次点赞合并计数)
    aggregate: bool,
}

/// 落库一条通知(可选同源聚合)。聚合时返回被更新后的旧行,否则返回新插入行。
async fn insert_notification(
    rb: &RBatis,
    spec: NotificationSpec,
) -> Result<SystemNotification, anyhow::Error> {
    let now = get_now_time_stamp_as_millis()?;
    let uuid = UuidNow::now_v7().to_string();
    let notification = SystemNotification {
        id: Some(Uuid::from_str(uuid.as_str())?),
        title: Some(spec.title.to_string()),
        content: Some(spec.content),
        created_at: Some(now),
        content_type: Some(0),
        user_id: Some(spec.user_id),
        biz_id: spec.biz_id,
        is_read: Some(false),
        level1: Some(spec.level1),
        level2: Some(spec.level2),
        level3: Some(spec.level3),
        level4: Some(spec.level4),
        unread_count: Some(1),
        priority: Some(spec.priority),
    };

    if spec.aggregate {
        Ok(SystemNotification::upsert_unread_aggregate(rb, &notification).await?)
    } else {
        SystemNotification::insert(rb, &notification).await?;
        Ok(notification)
    }
}

/// 通过内部 QUIC 服务实时推送系统通知给目标用户(PC 与移动端双端下发)。
///
/// 单端失败仅告警,不阻断另一端; 全部失败时返回最后一次错误供调用方记录。
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
        SYSTEM.to_string(),
    )?;

    let addr_str = read_global_config!("internal_quic_server", "address");
    let server_addr: SocketAddr = addr_str.parse()?;
    let preferred_index = compute_preferred_index(&target_id);

    let mut last_err = None;
    for platform in [PC_PLATFORM, MOBILE_PLATFORM] {
        let request = InternalQuicRequest {
            msg_type: NOTIFY_TYPE_MSG,
            payload: payload.clone(),
            target_user: target_id.clone(),
            preferred_index,
            platform: platform.to_string(),
            source: RequestSource::HttpApi,
            ttl: 3,
            close_after_delivery: false,
            incoming_session: String::new(),
            send_force_logout: false,
        };
        if let Err(e) = send_internal_quic_msg(server_addr, request).await {
            tracing::warn!(
                "系统通知实时推送失败 target={} platform={}: {}",
                target_id,
                platform,
                e
            );
            last_err = Some(e);
        }
    }
    match last_err {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

/// 新增好友请求通知(每次请求独立,不聚合)
pub async fn send_request_friend_msg(
    rb: &RBatis,
    user_id: rbatis::rbdc::Uuid,
    msg: String,
    biz_id: Option<String>,
) -> Result<SystemNotification, anyhow::Error> {
    insert_notification(
        rb,
        NotificationSpec {
            title: "好友通知",
            content: msg,
            user_id,
            biz_id,
            level1: NOTIFY_LEVEL1_SYSTEM,
            level2: NOTIFY_LEVEL2_FRIEND,
            level3: NOTIFY_LEVEL3_FRIEND_REQUEST,
            level4: 0,
            priority: 1,
            aggregate: false,
        },
    )
    .await
}

/// 新增好友处理通知(每次处理独立,不聚合)
pub async fn send_process_friend_msg(
    rb: &RBatis,
    user_id: rbatis::rbdc::Uuid,
    msg: String,
    biz_id: Option<String>,
) -> Result<SystemNotification, anyhow::Error> {
    insert_notification(
        rb,
        NotificationSpec {
            title: "好友通知",
            content: msg,
            user_id,
            biz_id,
            level1: NOTIFY_LEVEL1_SYSTEM,
            level2: NOTIFY_LEVEL2_FRIEND,
            level3: NOTIFY_LEVEL3_FRIEND_PROCESS,
            level4: 0,
            priority: 1,
            aggregate: false,
        },
    )
    .await
}

/// 新增群邀请通知(每次邀请独立,不聚合)
pub async fn send_group_invite_msg(
    rb: &RBatis,
    user_id: rbatis::rbdc::Uuid,
    msg: String,
    biz_id: Option<String>,
) -> Result<SystemNotification, anyhow::Error> {
    insert_notification(
        rb,
        NotificationSpec {
            title: "群邀请通知",
            content: msg,
            user_id,
            biz_id,
            level1: NOTIFY_LEVEL1_SYSTEM,
            level2: NOTIFY_LEVEL2_GROUP,
            level3: NOTIFY_LEVEL3_GROUP_INVITE,
            level4: 0,
            priority: 1,
            aggregate: false,
        },
    )
    .await
}

/// 群邀请处理结果通知（通知群主/管理员邀请已被处理,不聚合）
pub async fn send_group_invite_result_msg(
    rb: &RBatis,
    user_id: rbatis::rbdc::Uuid,
    msg: String,
    biz_id: Option<String>,
) -> Result<SystemNotification, anyhow::Error> {
    insert_notification(
        rb,
        NotificationSpec {
            title: "群邀请结果",
            content: msg,
            user_id,
            biz_id,
            level1: NOTIFY_LEVEL1_SYSTEM,
            level2: NOTIFY_LEVEL2_GROUP,
            level3: NOTIFY_LEVEL3_GROUP_INVITE_RESULT,
            level4: 0,
            priority: 1,
            aggregate: false,
        },
    )
    .await
}

/// 新增广场心动通知(同源未读聚合: 多次心动合并计数)
pub async fn send_plaza_like_msg(
    rb: &RBatis,
    user_id: rbatis::rbdc::Uuid,
    msg: String,
    biz_id: Option<String>,
) -> Result<SystemNotification, anyhow::Error> {
    insert_notification(
        rb,
        NotificationSpec {
            title: "交友广场通知",
            content: msg,
            user_id,
            biz_id,
            level1: NOTIFY_LEVEL1_SYSTEM,
            level2: NOTIFY_LEVEL2_PLAZA,
            level3: NOTIFY_LEVEL3_PLAZA_LIKE,
            level4: 0,
            priority: 1,
            aggregate: true,
        },
    )
    .await
}

/// 新增广场互相心动(匹配)通知(同源未读聚合)
pub async fn send_plaza_match_msg(
    rb: &RBatis,
    user_id: rbatis::rbdc::Uuid,
    msg: String,
    biz_id: Option<String>,
) -> Result<SystemNotification, anyhow::Error> {
    insert_notification(
        rb,
        NotificationSpec {
            title: "交友广场通知",
            content: msg,
            user_id,
            biz_id,
            level1: NOTIFY_LEVEL1_SYSTEM,
            level2: NOTIFY_LEVEL2_PLAZA,
            level3: NOTIFY_LEVEL3_PLAZA_MATCH,
            level4: 0,
            priority: 1,
            aggregate: true,
        },
    )
    .await
}

/// 新增动态点赞通知(同源未读聚合: 同一条动态的多次点赞合并计数)
pub async fn send_moment_like_msg(
    rb: &RBatis,
    user_id: rbatis::rbdc::Uuid,
    msg: String,
    biz_id: Option<String>,
) -> Result<SystemNotification, anyhow::Error> {
    insert_notification(
        rb,
        NotificationSpec {
            title: "动态通知",
            content: msg,
            user_id,
            biz_id,
            level1: NOTIFY_LEVEL1_SYSTEM,
            level2: NOTIFY_LEVEL2_MOMENT,
            level3: NOTIFY_LEVEL3_MOMENT_LIKE,
            level4: 0,
            priority: 1,
            aggregate: true,
        },
    )
    .await
}

/// 新增动态评论通知(同源未读聚合: 同一条动态的多次评论合并计数)
pub async fn send_moment_comment_msg(
    rb: &RBatis,
    user_id: rbatis::rbdc::Uuid,
    msg: String,
    biz_id: Option<String>,
) -> Result<SystemNotification, anyhow::Error> {
    insert_notification(
        rb,
        NotificationSpec {
            title: "动态通知",
            content: msg,
            user_id,
            biz_id,
            level1: NOTIFY_LEVEL1_SYSTEM,
            level2: NOTIFY_LEVEL2_MOMENT,
            level3: NOTIFY_LEVEL3_MOMENT_COMMENT,
            level4: 0,
            priority: 1,
            aggregate: true,
        },
    )
    .await
}

/// 获取用户未读的通知（仅最近 NOTIFY_RETENTION_DAYS 天内,兼容旧客户端）
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

/// 通知中心列表: keyset 分页 + 分类/已读过滤,返回列表、下一页游标与未读总数。
pub async fn list_notifications(
    rb: &RBatis,
    user_id: Option<String>,
    dto: NotificationListDTO,
) -> Result<String, anyhow::Error> {
    let user_id =
        rbatis::rbdc::Uuid::from_str(user_id.ok_or(anyhow!("user_id is empty"))?.as_str())?;
    let now = get_now_time_stamp_as_millis()?;
    let since = now - NOTIFY_RETENTION_DAYS * 24 * 3600 * 1000;
    let size = dto.size.unwrap_or(NOTIFY_LIST_DEFAULT_SIZE).clamp(1, NOTIFY_LIST_MAX_SIZE);
    let cursor = match dto.cursor {
        Some(c) => Some((c.created_at, rbatis::rbdc::Uuid::from_str(&c.id)?)),
        None => None,
    };

    // limit+1 探测 has_more
    let (cursor_at, cursor_id) = match cursor {
        Some((at, id)) => (Some(at), Some(id)),
        None => (None, None),
    };
    let mut rows = SystemNotification::select_page(
        rb,
        NotificationPageQuery {
            user_id: user_id.clone(),
            since,
            is_read: dto.is_read,
            level1: dto.level1,
            level2: dto.level2,
            cursor_at,
            cursor_id,
            size: size + 1,
        },
    )
    .await?;
    let has_more = rows.len() > size as usize;
    rows.truncate(size as usize);

    let next_cursor = if has_more {
        rows.last().and_then(|n| {
            Some(NotificationCursor { created_at: n.created_at?, id: n.id.as_ref()?.to_string() })
        })
    } else {
        None
    };

    let unread_total = SystemNotification::count_unread_total(rb, &user_id, since).await?;
    let vo = NotificationListVO {
        notifications: rows.iter().map(SystemNotificationVO::from).collect(),
        has_more,
        next_cursor,
        unread_total,
    };
    Ok(CommonResponseRef::<NotificationListVO>::success_json(&vo)?)
}

/// 未读数汇总(角标 + 分类 tab): 总数 + 按 (level1, level2) 分组
pub async fn get_unread_summary(
    rb: &RBatis,
    user_id: Option<String>,
) -> Result<String, anyhow::Error> {
    let user_id =
        rbatis::rbdc::Uuid::from_str(user_id.ok_or(anyhow!("user_id is empty"))?.as_str())?;
    let now = get_now_time_stamp_as_millis()?;
    let since = now - NOTIFY_RETENTION_DAYS * 24 * 3600 * 1000;

    let total = SystemNotification::count_unread_total(rb, &user_id, since).await?;
    let groups = SystemNotification::count_unread_grouped(rb, &user_id, since)
        .await?
        .into_iter()
        .map(|g| NotificationLevelCountVO { level1: g.level1, level2: g.level2, count: g.count })
        .collect();
    let vo = NotificationUnreadSummaryVO { total, groups };
    Ok(CommonResponseRef::<NotificationUnreadSummaryVO>::success_json(&vo)?)
}

/// 批量标记已读: 指定 id 时按 id 标记; ids 为空时按分类过滤全部已读。
pub async fn mark_notifications_read(
    rb: &RBatis,
    user_id: Option<String>,
    dto: MarkReadDTO,
) -> Result<String, anyhow::Error> {
    let user_id =
        rbatis::rbdc::Uuid::from_str(user_id.ok_or(anyhow!("user_id is empty"))?.as_str())?;
    if dto.ids.is_empty() {
        mark_all_read(rb, &user_id, dto.level1, dto.level2).await?;
    } else {
        mark_read_by_ids(rb, &user_id, &dto.ids).await?;
    }
    Ok(CommonResponseNoDataRef::success_empty())
}

impl From<&SystemNotification> for SystemNotificationVO {
    fn from(n: &SystemNotification) -> Self {
        Self {
            id: n.id.as_ref().map(|u| u.to_string()).unwrap_or_default(),
            title: n.title.clone(),
            content: n.content.clone(),
            created_at: n.created_at,
            content_type: n.content_type,
            user_id: n.user_id.as_ref().map(|u| u.to_string()),
            is_read: n.is_read,
            biz_id: n.biz_id.clone(),
            level1: n.level1,
            level2: n.level2,
            level3: n.level3,
            level4: n.level4,
            unread_count: n.unread_count,
            priority: n.priority,
        }
    }
}
