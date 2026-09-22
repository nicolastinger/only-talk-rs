use anyhow::anyhow;
use common::models::chat_entity::chat_message_record::ChatMessageRecord;
use common::models::group_entity::group_message_record::GroupMessageRecord;
use common::models::session_entity::aggregate::aggregate_user_sessions;
use common::models::session_entity::session::{SESSION_TYPE_GROUP, SESSION_TYPE_SINGLE, Session};
use common::models::session_entity::user_session::UserSession;
use common::utils::time::get_now_time_stamp_as_millis;
use rbatis::RBatis;
use rbatis::rbdc::Uuid;
use tracing::{info, warn};

use crate::http_service::session_service::dto::{
    SessionControlDTO, SessionListCursor, SessionListDTO, SessionListResponseVO, SessionReadDTO,
    SessionSyncedDTO, SessionVO, SyncMessageVO, SyncRequestDTO, SyncResponseVO, SyncSessionVO,
};
use crate::utils::http_response::CommonResponseNoDataRef;

/// 会话列表默认页大小。
const SESSION_LIST_DEFAULT_SIZE: u32 = 50;
/// 会话列表页大小硬上限。
const SESSION_LIST_MAX_SIZE: u32 = 100;

/// 离线消息最大回溯窗口(天)。服务端计算, 不信任客户端时间(主方案 §8.1)。
/// 与每日清理共用 `common::config_str::SYNC_WINDOW_DAYS`(任务09 §3.3, 避免双源漂移)。
const SYNC_WINDOW_DAYS: i64 = common::config_str::SYNC_WINDOW_DAYS;
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

/// 离线同步(任务12 正向追平): 无状态窗口查询, 拉取起点 = 客户端本地已同步的最新 id。
///
/// - 归属校验: 无 `user_session` 行 → 静默跳过(warn), 不报错(兼容聚合未跑/已退群, 不泄露会话存在性);
///   同一次查询顺带取出 `session_type`(零额外成本)。
/// - `sessions` 空/缺省 → 空响应(不再服务端筛会话)。
/// - 查询形态唯一: `session_uuid=? AND id>after AND "timestamp">boundary ORDER BY id ASC LIMIT limit+1`;
///   取前 limit 条(已升序); `has_more` = limit+1 探测; `next_cursor` = 末条 id(空批 = after)。
///   7 天窗口由 `boundary` 过滤天然达成 —— 客户端拉到 `has_more=false` 即追平服务端最新 id 或窗口尽头。
pub async fn sync_sessions(
    rb: &RBatis,
    me: Option<String>,
    dto: SyncRequestDTO,
) -> Result<SyncResponseVO, anyhow::Error> {
    let me: Uuid = me.ok_or_else(|| anyhow!("账号获取失败"))?.parse()?;
    let now = get_now_time_stamp_as_millis()?;
    let boundary = window_boundary(now);
    let limit = dto.limit.unwrap_or(SYNC_DEFAULT_LIMIT).clamp(1, SYNC_MAX_LIMIT);

    let mut sessions_out = Vec::new();
    for req in dto.sessions.unwrap_or_default() {
        // rbdc::Uuid::from_str 不做校验(永远成功), 必须用 uuid::Uuid::parse_str 兜住非法值。
        let Ok(parsed) = uuid::Uuid::parse_str(&req.session_uuid) else {
            warn!("[session/sync] 非法 session_uuid: {}", req.session_uuid);
            continue;
        };
        let session_uuid: Uuid = parsed.to_string().parse()?;

        // 归属校验 + session_type(一次查询两用); 无行 → 静默跳过
        let Some(row) = UserSession::select_by_user_and_session(rb, &me, &session_uuid).await?
        else {
            warn!("[session/sync] 会话不属于当前用户或未聚合, 跳过: {}", req.session_uuid);
            continue;
        };
        let session_type = row.session_type.unwrap_or(SESSION_TYPE_SINGLE);
        let after = req.after_id.unwrap_or(0);

        // 单一查询: 正向翻页(已升序), limit+1 探测 has_more
        let (messages, has_more) = if session_type == SESSION_TYPE_GROUP {
            let raw = GroupMessageRecord::select_window_after(
                rb,
                &session_uuid,
                after,
                boundary,
                limit + 1,
            )
            .await?;
            let has_more = raw.len() > limit as usize;
            let msgs: Vec<SyncMessageVO> = raw
                .into_iter()
                .take(limit as usize)
                .map(|m| group_msg_to_vo(m, &session_uuid))
                .collect();
            (msgs, has_more)
        } else {
            let raw = ChatMessageRecord::select_window_after(
                rb,
                &session_uuid,
                after,
                boundary,
                limit + 1,
            )
            .await?;
            let has_more = raw.len() > limit as usize;
            let msgs: Vec<SyncMessageVO> = raw
                .into_iter()
                .take(limit as usize)
                .map(|m| single_msg_to_vo(m, &session_uuid))
                .collect();
            (msgs, has_more)
        };

        // 空批 next_cursor = 请求的 after(无新消息), 客户端据此感知已追平
        let next_cursor = messages.last().map(|m| m.id).unwrap_or(after);
        sessions_out.push(SyncSessionVO {
            session_uuid: session_uuid.to_string(),
            session_type,
            messages,
            next_cursor,
            has_more,
        });
    }

    info!("[session/sync] 同步完成: user={}, {} 个会话", me, sessions_out.len());
    Ok(SyncResponseVO { server_time: now, sessions: sessions_out })
}

/// 会话列表(任务06 §9.1): keyset 分页 + 未读现算 + 软删复活。
///
/// 读路径惰性聚合: 首页(无游标)先跑一次 `aggregate_user_sessions`, 把消息表最新状态
/// 收敛进 `session.last_message_*`(幂等, 失败不阻塞)。客户端登录 / 断线重连必然发起
/// 本接口首页 —— 无读者就不聚合, quic_service 生产者不再维护会话状态。
pub async fn list_sessions(
    rb: &RBatis,
    me: Option<String>,
    dto: SessionListDTO,
) -> Result<SessionListResponseVO, anyhow::Error> {
    let me_uuid: Uuid = me.ok_or_else(|| anyhow!("账号获取失败"))?.parse()?;
    let size = dto.size.unwrap_or(SESSION_LIST_DEFAULT_SIZE).clamp(1, SESSION_LIST_MAX_SIZE);

    // 首页才聚合: 翻页复用本页已收敛状态, 避免每页重复点查
    if dto.cursor.is_none() {
        let _ = aggregate_user_sessions(rb, &me_uuid).await;
    }

    // 解析游标(首页为 None); 非法 uuid 用 uuid::Uuid 校验(rbdc::Uuid::from_str 不校验)
    let cursor = match dto.cursor {
        Some(c) => {
            let Ok(parsed) = uuid::Uuid::parse_str(&c.session_uuid) else {
                return Err(anyhow!("游标 session_uuid 非法: {}", c.session_uuid));
            };
            Some((c.pinned, c.last_message_at, parsed.to_string().parse()?))
        }
        None => None,
    };

    // size+1 探测 has_more(与 /session/sync 同款手法)
    let mut rows = Session::select_list(rb, &me_uuid, cursor, size + 1).await?;
    let has_more = rows.len() > size as usize;
    rows.truncate(size as usize);

    let sessions: Vec<SessionVO> = rows
        .into_iter()
        .map(|r| SessionVO {
            session_uuid: r.session_uuid.to_string(),
            session_type: r.session_type.unwrap_or(SESSION_TYPE_SINGLE),
            peer_uuid: r.peer_uuid.map(|p| p.to_string()),
            last_message_id: r.last_message_id.unwrap_or(0),
            last_message_at: r.last_message_at.unwrap_or(0),
            last_preview: r.last_preview.unwrap_or_default(),
            pinned: r.pinned.unwrap_or(0),
            muted: r.muted.unwrap_or(0),
            unread: r.unread.unwrap_or(0),
        })
        .collect();

    // next_cursor = 本页最后一行(客户端原样回传三元组)
    let next_cursor = if has_more {
        sessions.last().map(|s| SessionListCursor {
            pinned: s.pinned,
            last_message_at: s.last_message_at,
            session_uuid: s.session_uuid.clone(),
        })
    } else {
        None
    };

    info!("[session/list] user={}, 返回 {} 个会话, has_more={}", me_uuid, sessions.len(), has_more);
    Ok(SessionListResponseVO { sessions, has_more, next_cursor })
}

/// 控制信息公共前置: 解析 me / session_uuid, 并顺带聚合(§6.1 触发表: 刷新
/// `session.last_message_*`, 保证软删 push-to-bottom 用到的 `last_message_id` 新鲜; 失败不阻塞)。
async fn prepare_control(
    rb: &RBatis,
    me: Option<String>,
    dto: &SessionControlDTO,
) -> Result<(Uuid, Uuid), anyhow::Error> {
    let me_uuid: Uuid = me.ok_or_else(|| anyhow!("账号获取失败"))?.parse()?;
    let Ok(parsed) = uuid::Uuid::parse_str(&dto.session_uuid) else {
        return Err(anyhow!("session_uuid 非法: {}", dto.session_uuid));
    };
    let s: Uuid = parsed.to_string().parse()?;
    let _ = aggregate_user_sessions(rb, &me_uuid).await;
    Ok((me_uuid, s))
}

/// 置顶/取消置顶(控制信息)。
pub async fn pin_session(
    rb: &RBatis,
    me: Option<String>,
    dto: SessionControlDTO,
) -> Result<(), anyhow::Error> {
    let (me_uuid, s) = prepare_control(rb, me, &dto).await?;
    let affected = UserSession::update_pinned(rb, &me_uuid, &s, dto.value.unwrap_or(1)).await?;
    if affected == 0 {
        return Err(anyhow!("会话不存在: {}", dto.session_uuid));
    }
    Ok(())
}

/// 免打扰/取消(控制信息)。
pub async fn mute_session(
    rb: &RBatis,
    me: Option<String>,
    dto: SessionControlDTO,
) -> Result<(), anyhow::Error> {
    let (me_uuid, s) = prepare_control(rb, me, &dto).await?;
    let affected = UserSession::update_muted(rb, &me_uuid, &s, dto.value.unwrap_or(1)).await?;
    if affected == 0 {
        return Err(anyhow!("会话不存在: {}", dto.session_uuid));
    }
    Ok(())
}

/// 软删会话(控制信息): `deleted_at` 置当前时刻并把 `last_read_id` 推到底。
pub async fn delete_session(
    rb: &RBatis,
    me: Option<String>,
    dto: SessionControlDTO,
) -> Result<(), anyhow::Error> {
    let (me_uuid, s) = prepare_control(rb, me, &dto).await?;
    let affected = UserSession::soft_delete(rb, &me_uuid, &s).await?;
    if affected == 0 {
        return Err(anyhow!("会话不存在: {}", dto.session_uuid));
    }
    Ok(())
}
