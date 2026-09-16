use rbatis::crud;
use rbatis::rbdc::Uuid;
use serde::{Deserialize, Serialize};

/// 用户视角的会话状态: 每用户每会话一行。
///
/// 不变式: `last_read_id <= synced_id`(服务端在上报处钳制)。
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct UserSession {
    pub id: Option<i64>,
    /// 谁的列表
    pub user_uuid: Uuid,
    /// 关联 session
    pub session_uuid: Uuid,
    /// 冗余会话类型, 免 join
    pub session_type: Option<i16>,
    /// 单聊: 对方; 群聊: NULL
    pub peer_uuid: Option<Uuid>,
    /// 已读游标(角标口径), 只前进
    pub last_read_id: Option<i64>,
    /// 同步游标(拉取起点), 只前进, 恒 >= last_read_id
    pub synced_id: Option<i64>,
    /// 置顶: 0-否 1-是
    pub pinned: Option<i16>,
    /// 免打扰: 0-否 1-是
    pub muted: Option<i16>,
    /// 软删时刻(毫秒); session.last_message_at > 此值即视为复活
    pub deleted_at: Option<i64>,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
}

crud!(UserSession {});
