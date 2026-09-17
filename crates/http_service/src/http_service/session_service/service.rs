use anyhow::anyhow;
use common::models::session_entity::user_session::UserSession;
use rbatis::RBatis;
use rbatis::rbdc::Uuid;
use tracing::{info, warn};

use crate::http_service::session_service::dto::{SessionReadDTO, SessionSyncedDTO};
use crate::utils::http_response::CommonResponseNoDataRef;

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
