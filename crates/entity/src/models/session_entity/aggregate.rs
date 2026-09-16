//! 会话聚合: 在用户上线/下线时把消息表的最新状态收敛进 `session` / `user_session` 表。
//!
//! 聚合是"最终一致"的兜底: 消息落库仍走同步直写, 会话列表状态由本任务在
//! 用户上线/下线(以及任务 09 的定时兜底)时收敛。全部写路径单调或幂等,
//! 重复执行/多节点并发执行结果一致。

use rbatis::executor::Executor;
use rbatis::rbdc::Uuid;
use tracing::info;

use crate::models::chat_entity::chat_message_record::ChatMessageRecord;
use crate::models::group_entity::group_member::GroupMember;
use crate::models::group_entity::group_message_record::GroupMessageRecord;
use crate::models::session_entity::session::{SESSION_TYPE_GROUP, SESSION_TYPE_SINGLE, Session};
use crate::models::session_entity::user_session::UserSession;

/// 聚合结果(仅用于日志观测)。
#[derive(Debug, Default)]
pub struct AggregateReport {
    pub single_sessions: usize,
    pub group_sessions: usize,
    pub user_session_upserted: usize,
}

/// 聚合指定用户的全部会话(§6.1 后台聚合任务)。
///
/// - 单聊: 从消息表发现会话 → `session` upsert + 单调更新 → 建 me 视角的 `user_session`(含 peer)
/// - 群聊: 从群成员表发现 → `session` upsert + 单调更新 → 建 me 视角的 `user_session`
/// - 只建 me 自己的 `user_session` 行; 对方的行在对方聚合时收敛
/// - 幂等: 重复执行结果一致(全部写路径单调 / `DO NOTHING`)
pub async fn aggregate_user_sessions(
    rb: &dyn Executor,
    me: &Uuid,
) -> Result<AggregateReport, rbatis::Error> {
    let mut report = AggregateReport::default();

    // 1. 单聊
    let latest = ChatMessageRecord::latest_per_session_for_user(rb, me).await?;
    report.single_sessions = latest.len();
    for msg in latest {
        let session_uuid = msg.session_uuid.clone();
        let peer = if msg.send_user == *me { msg.recv_user.clone() } else { msg.send_user.clone() };

        Session::upsert(
            rb,
            &Session {
                session_uuid: session_uuid.clone(),
                session_type: Some(SESSION_TYPE_SINGLE),
                last_message_id: None,
                last_message_at: None,
                last_preview: None,
                created_at: None,
                updated_at: None,
            },
        )
        .await?;
        Session::update_last_message(
            rb,
            &session_uuid,
            msg.id.unwrap_or(0),
            msg.timestamp.unwrap_or(0),
            None, // preview 生成策略见任务 06(§3.4)
        )
        .await?;

        UserSession::upsert(
            rb,
            &UserSession {
                id: None,
                user_uuid: me.clone(),
                session_uuid,
                session_type: Some(SESSION_TYPE_SINGLE),
                peer_uuid: Some(peer),
                last_read_id: None,
                synced_id: None,
                pinned: None,
                muted: None,
                deleted_at: None,
                created_at: None,
                updated_at: None,
            },
        )
        .await?;
        report.user_session_upserted += 1;
    }

    // 2. 群聊
    let memberships = GroupMember::select_groups_by_user(rb, me).await?;
    report.group_sessions = memberships.len();
    for m in memberships {
        let Some(group_uuid) = m.group_uuid else { continue };
        Session::upsert(
            rb,
            &Session {
                session_uuid: group_uuid.clone(),
                session_type: Some(SESSION_TYPE_GROUP),
                last_message_id: None,
                last_message_at: None,
                last_preview: None,
                created_at: None,
                updated_at: None,
            },
        )
        .await?;
        if let Some(latest_msg) =
            GroupMessageRecord::select_latest_by_group(rb, &group_uuid).await?
        {
            Session::update_last_message(
                rb,
                &group_uuid,
                latest_msg.id.unwrap_or(0),
                latest_msg.timestamp.unwrap_or(0),
                None,
            )
            .await?;
        }
        // 群聊 user_session 走 init_for_group_join 语义(游标初始化), 对已存在行为幂等
        UserSession::init_for_group_join(rb, me, &group_uuid).await?;
        report.user_session_upserted += 1;
    }

    info!(
        "[session] 聚合完成: user={}, 单聊会话={}, 群会话={}",
        me, report.single_sessions, report.group_sessions
    );
    Ok(report)
}
