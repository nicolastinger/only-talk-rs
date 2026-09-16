use rbatis::crud;
use rbatis::rbdc::Uuid;
use serde::{Deserialize, Serialize};

/// 会话类型常量与 common::config_str 保持一致(避免 entity 反向依赖 common)。
/// 注意依赖方向: common → entity, 因此常量定义在 entity, common 侧 re-export 或直接引用。
pub const SESSION_TYPE_SINGLE: i16 = 1;
pub const SESSION_TYPE_GROUP: i16 = 2;
pub const SESSION_TYPE_SYSTEM: i16 = 3;
pub const SESSION_TYPE_OFFICIAL_ACCOUNT: i16 = 4;

/// 会话本体: 单聊=用户对, 群聊=群; 双方/全员共享一行。
///
/// `last_message_*` 由后台聚合任务维护(用户上线/下线/控制信息变更时), 非实时更新。
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct Session {
    pub session_uuid: Uuid,
    /// 1-单聊 2-群聊 3-系统 4-公众号
    pub session_type: Option<i16>,
    /// 最后一条消息 id(消息表 id, 只前进不回退)
    pub last_message_id: Option<i64>,
    /// 最后一条消息时间(毫秒)
    pub last_message_at: Option<i64>,
    /// 列表摘要(已截断至 256 字符)
    pub last_preview: Option<String>,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
}

crud!(Session {});
