//! 会话聚合: 在会话列表被读取时(读路径)把消息表的最新状态收敛进 `session` 表。
//!
//! 触发点(全在 http_service 读路径, 不在 quic_service 生产者):
//! - `/session/list` 首页(客户端登录 / 断线重连时必然发起)
//! - 控制信息变更前(`prepare_control`, 保证软删 push-to-bottom 的 `last_message_id` 新鲜)
//!
//! 单聊: 04b 好友通过即建双方 `user_session` 行, 聚合仅按这些行逐会话点查最新消息;
//! 群聊: 群成员关系(membership)即权威, 聚合按成员表逐群点查。
//! 全部写路径单调或幂等, 重复执行/多节点并发执行结果一致。
//! 无消息的会话跳过(不把已收敛的 `last_message_*` 清零)。

use rbatis::executor::Executor;
use rbatis::rbdc::{Bytes, Uuid};
use tracing::info;

use crate::models::chat_entity::chat_message_record::ChatMessageRecord;
use crate::models::group_entity::group_member::GroupMember;
use crate::models::group_entity::group_message_record::GroupMessageRecord;
use crate::models::session_entity::session::{SESSION_TYPE_GROUP, SESSION_TYPE_SINGLE, Session};
use crate::models::session_entity::user_session::UserSession;

/// 列表摘要最大字符数。
const PREVIEW_MAX_CHARS: usize = 256;

/// 由消息 raw 生成列表摘要(任务06 §2): 内容即 UTF-8 文本, `chars().take` 保证不切半个多字节字符。
fn preview_from_raw(raw: &Bytes) -> String {
    String::from_utf8_lossy(raw.as_ref()).chars().take(PREVIEW_MAX_CHARS).collect()
}

/// 聚合结果(仅用于日志观测)。
#[derive(Debug, Default)]
pub struct AggregateReport {
    pub single_sessions: usize,
    pub group_sessions: usize,
    pub user_session_upserted: usize,
}

/// 聚合指定用户的全部会话(§6.1 后台聚合任务)。
///
/// - 单聊: 从 `user_session` 行发现会话(04b 好友通过即建行, 行存在是关系成立的必然) →
///   `session` upsert + 单调更新; 无最新消息的会话跳过
/// - 群聊: 从群成员表发现 → `session` upsert + 单调更新 → 建 me 视角的 `user_session`
///   (`init_for_group_join` 语义, 幂等)
/// - 幂等: 重复执行结果一致(全部写路径单调 / `DO NOTHING`)
pub async fn aggregate_user_sessions(
    rb: &dyn Executor,
    me: &Uuid,
) -> Result<AggregateReport, rbatis::Error> {
    let mut report = AggregateReport::default();

    // 1. 单聊: user_session 驱动(不再扫消息表发现会话; 发现职责已由任务 04b 的建行钩子承接)
    let user_sessions = UserSession::select_by_user(rb, me).await?;
    for us in user_sessions {
        // 群聊走成员表驱动(见下); 防御性跳过 session_type 缺失的异常行
        if us.session_type == Some(SESSION_TYPE_GROUP) {
            continue;
        }
        // 无消息的会话(如刚建行的新关系)跳过 —— 不把已收敛状态清零
        let Some(latest) = ChatMessageRecord::latest_by_session(rb, &us.session_uuid).await? else {
            continue;
        };
        let session_uuid = us.session_uuid.clone();

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
            latest.id.unwrap_or(0),
            latest.timestamp.unwrap_or(0),
            Some(&preview_from_raw(&latest.raw)), // preview 生成策略见任务 06 §2
        )
        .await?;
        report.single_sessions += 1;
    }

    // 2. 群聊: membership 驱动(群成员关系即权威, 逐群点查)
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
                Some(&preview_from_raw(&latest_msg.raw)),
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
