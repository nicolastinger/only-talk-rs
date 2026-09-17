use anyhow::anyhow;
use common::models::chat_entity::chat_message_record::ChatMessageRecord;
use common::models::group_entity::group_message_record::GroupMessageRecord;
use common::models::session_entity::session::{SESSION_TYPE_GROUP, SESSION_TYPE_SINGLE};
use common::models::session_entity::user_session::UserSession;
use common::utils::time::get_now_time_stamp_as_millis;
use rbatis::RBatis;
use rbatis::rbdc::Uuid;
use tracing::{info, warn};

use crate::http_service::session_service::dto::{
    SessionReadDTO, SessionSyncedDTO, SyncMessageVO, SyncRequestDTO, SyncResponseVO, SyncSessionVO,
};
use crate::utils::http_response::CommonResponseNoDataRef;

/// 离线消息最大回溯窗口(天)。服务端计算, 不信任客户端时间(主方案 §8.1)。
const SYNC_WINDOW_DAYS: i64 = 7;
/// 单会话单次同步默认条数。
const SYNC_DEFAULT_LIMIT: u32 = 100;
/// 单会话单次同步硬上限(防客户端传大值)。
const SYNC_MAX_LIMIT: u32 = 200;

/// 7 天窗口边界(毫秒)。
fn window_boundary(now: i64) -> i64 {
    now - SYNC_WINDOW_DAYS * 24 * 3600 * 1000
}

/// 单聊消息 → 统一 VO。
fn single_msg_to_vo(m: ChatMessageRecord, session_uuid: &Uuid) -> SyncMessageVO {
    SyncMessageVO {
        id: m.id.unwrap_or(0),
        nano_id: m.nano_id.unwrap_or_default(),
        session_uuid: session_uuid.to_string(),
        session_type: SESSION_TYPE_SINGLE,
        send_user: m.send_user.to_string(),
        recv_user: m.recv_user.to_string(),
        text_type: m.text_type.unwrap_or(0) as u16,
        timestamp: m.timestamp.unwrap_or(0),
        raw: m.raw.into_inner(),
    }
}

/// 群消息 → 统一 VO(recv_user = group_uuid, text_type = msg_type)。
fn group_msg_to_vo(m: GroupMessageRecord, session_uuid: &Uuid) -> SyncMessageVO {
    SyncMessageVO {
        id: m.id.unwrap_or(0),
        nano_id: m.nano_id.unwrap_or_default(),
        session_uuid: session_uuid.to_string(),
        session_type: SESSION_TYPE_GROUP,
        send_user: m.send_user.map(|u| u.to_string()).unwrap_or_default(),
        recv_user: session_uuid.to_string(),
        text_type: m.msg_type.unwrap_or(0) as u16,
        timestamp: m.timestamp.unwrap_or(0),
        raw: m.raw.into_inner(),
    }
}

/// 已读上报(§7.1): 逐条推进, 钳制与只前进在数据层保证(任务03)。
///
/// 非法 `session_uuid` 记 warn 跳过, 不整体失败(部分成功优于全部拒绝)。
pub async fn report_session_read(
    rb: &RBatis,
    me: Option<String>,
    dto: SessionReadDTO,
) -> Result<String, anyhow::Error> {
    let me: Uuid = me.ok_or_else(|| anyhow!("账号获取失败"))?.parse()?;
    let total = dto.reads.len();
    for item in dto.reads {
        let Ok(session_uuid) = item.session_uuid.parse::<Uuid>() else {
            warn!("[session/read] 非法 session_uuid: {}", item.session_uuid);
            continue;
        };
        let affected =
            UserSession::update_last_read_id(rb, &me, &session_uuid, item.last_read_id).await?;
        if affected == 0 {
            // 行不存在(未聚合)或无需推进 —— 前者记日志便于排查
            warn!(
                "[session/read] 无效上报(行不存在或未推进): user={}, session={}",
                me, session_uuid
            );
        }
    }
    info!("[session/read] 上报完成: user={}, {} 条", me, total);
    Ok(CommonResponseNoDataRef::success_empty())
}

/// 同步游标回报(§7.3): 逐条推进, 只前进。
pub async fn report_session_synced(
    rb: &RBatis,
    me: Option<String>,
    dto: SessionSyncedDTO,
) -> Result<String, anyhow::Error> {
    let me: Uuid = me.ok_or_else(|| anyhow!("账号获取失败"))?.parse()?;
    let total = dto.sessions.len();
    for item in dto.sessions {
        let Ok(session_uuid) = item.session_uuid.parse::<Uuid>() else {
            warn!("[session/synced] 非法 session_uuid: {}", item.session_uuid);
            continue;
        };
        let _ = UserSession::update_synced_id(rb, &me, &session_uuid, item.synced_id).await?;
    }
    info!("[session/synced] 回报完成: user={}, {} 条", me, total);
    Ok(CommonResponseNoDataRef::success_empty())
}

/// 离线同步(任务05 §8.1): 增量 / initial 双模式。
///
/// - 拉取起点永远是服务端 `user_session.synced_id`(请求不带游标);
/// - `sessions` 空 → 服务端筛选全部会话, 增量模式下无未同步消息的会话不输出(空响应 = 同步完毕);
/// - 增量用 `limit+1` 探测 `has_more`; `truncated_by_window` 仅在 `has_more=false` 且非 initial 时探测。
pub async fn sync_sessions(
    rb: &RBatis,
    me: Option<String>,
    dto: SyncRequestDTO,
) -> Result<SyncResponseVO, anyhow::Error> {
    let me: Uuid = me.ok_or_else(|| anyhow!("账号获取失败"))?.parse()?;
    let now = get_now_time_stamp_as_millis()?;
    let boundary = window_boundary(now);
    let limit = dto.limit.unwrap_or(SYNC_DEFAULT_LIMIT).clamp(1, SYNC_MAX_LIMIT);
    let initial = dto.mode.as_deref() == Some("initial");

    // 1. 目标会话行
    let rows: Vec<UserSession> = match dto.sessions.as_deref() {
        None | Some([]) => UserSession::select_by_user(rb, &me).await?,
        Some(ids) => {
            // 指定会话: 逐个解析并查行(唯一键含 user_uuid, 查询天然按 user 过滤, 无越权可能);
            // 非法 / 不存在的跳过, 不整体失败。
            // 注意: rbdc::Uuid::from_str 不做校验(永远成功), 必须用 uuid::Uuid::parse_str 兜住非法值。
            let mut v = Vec::new();
            for id in ids {
                let Ok(parsed) = uuid::Uuid::parse_str(id) else {
                    warn!("[session/sync] 非法 session_uuid: {id}");
                    continue;
                };
                let u: Uuid = parsed.to_string().parse()?;
                if let Some(row) = UserSession::select_by_user_and_session(rb, &me, &u).await? {
                    v.push(row);
                }
            }
            v
        }
    };

    // 2. 逐会话拉取(串行; 会话数 × 单分区索引扫)
    let mut sessions_out = Vec::new();
    for row in rows {
        let session_type = row.session_type.unwrap_or(SESSION_TYPE_SINGLE);
        let session_uuid = row.session_uuid;

        let (messages, has_more, cursor_for_truncated) = if initial {
            let msgs = if session_type == SESSION_TYPE_GROUP {
                GroupMessageRecord::select_latest_in_window_by_group(
                    rb,
                    &session_uuid,
                    boundary,
                    limit,
                )
                .await?
                .into_iter()
                .map(|m| group_msg_to_vo(m, &session_uuid))
                .collect::<Vec<_>>()
            } else {
                ChatMessageRecord::select_latest_by_session(rb, &session_uuid, boundary, limit)
                    .await?
                    .into_iter()
                    .map(|m| single_msg_to_vo(m, &session_uuid))
                    .collect::<Vec<_>>()
            };
            (msgs, false, 0)
        } else {
            let cursor = row.synced_id.unwrap_or(0);
            // 两组查询返回不同记录类型, 分支内各自转 VO, 避免类型统一问题
            let (msgs, has_more): (Vec<SyncMessageVO>, bool) = if session_type == SESSION_TYPE_GROUP
            {
                let raw =
                    GroupMessageRecord::select_sync(rb, &session_uuid, cursor, boundary, limit + 1)
                        .await?;
                let has_more = raw.len() > limit as usize;
                let msgs = raw
                    .into_iter()
                    .take(limit as usize)
                    .map(|m| group_msg_to_vo(m, &session_uuid))
                    .collect();
                (msgs, has_more)
            } else {
                let raw =
                    ChatMessageRecord::select_sync(rb, &session_uuid, cursor, boundary, limit + 1)
                        .await?;
                let has_more = raw.len() > limit as usize;
                let msgs = raw
                    .into_iter()
                    .take(limit as usize)
                    .map(|m| single_msg_to_vo(m, &session_uuid))
                    .collect();
                (msgs, has_more)
            };
            if msgs.is_empty() {
                continue; // 增量: 无未同步消息的会话不输出(空响应 = 同步完毕)
            }
            (msgs, has_more, cursor)
        };

        // 3. 截断探测: 仅增量且 has_more=false 时(还有更多可拉时无意义)
        let truncated = if has_more || initial {
            false
        } else if session_type == SESSION_TYPE_GROUP {
            GroupMessageRecord::exists_beyond_window(
                rb,
                &session_uuid,
                cursor_for_truncated,
                boundary,
            )
            .await?
        } else {
            ChatMessageRecord::exists_beyond_window(
                rb,
                &session_uuid,
                cursor_for_truncated,
                boundary,
            )
            .await?
        };

        let next_cursor = messages.last().map(|m| m.id).unwrap_or(0);
        sessions_out.push(SyncSessionVO {
            session_uuid: session_uuid.to_string(),
            session_type,
            messages,
            next_cursor,
            has_more,
            truncated_by_window: truncated,
        });
    }

    info!(
        "[session/sync] 同步完成: user={}, mode={}, {} 个会话",
        me,
        if initial { "initial" } else { "incremental" },
        sessions_out.len()
    );
    Ok(SyncResponseVO { server_time: now, sessions: sessions_out })
}
