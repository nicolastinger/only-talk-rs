use rbatis::rbdc::Uuid;
use serde::{Deserialize, Serialize};

/// 会话类型: 单聊
pub const CHAT_TYPE_SINGLE: u32 = 1;
/// 会话类型: 群聊
pub const CHAT_TYPE_GROUP: u32 = 2;

/// 客户端上报已读消息的 DTO（单聊与群聊共用同一上报接口，按 chat_type 分流）
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct AddReadChatRecordDTO {
    pub nano_id: Option<String>,
    pub timestamp: Option<i64>,
    pub send_user: Uuid,
    pub recv_user: Uuid,
    /// 会话类型 (1: 单聊, 2: 群聊)，默认单聊
    #[serde(default)]
    pub chat_type: Option<u32>,
}
