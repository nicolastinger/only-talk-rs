use serde::{Deserialize, Serialize};

/// 广播类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BroadcastType {
    GroupText,
    GroupImage,
    GroupFile,
    GroupNotification,
}

impl BroadcastType {
    pub fn from_msg_type(msg_type: u16) -> Self {
        match msg_type {
            10 => BroadcastType::GroupText,
            11 => BroadcastType::GroupImage,
            12 => BroadcastType::GroupFile,
            _ => BroadcastType::GroupNotification,
        }
    }

    pub fn to_msg_type(&self) -> u16 {
        match self {
            BroadcastType::GroupText => 10,
            BroadcastType::GroupImage => 11,
            BroadcastType::GroupFile => 12,
            BroadcastType::GroupNotification => 13,
        }
    }
}

/// 内网 QUIC 群消息广播请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InternalGroupBroadcast {
    /// 消息类型标识
    pub broadcast_type: BroadcastType,
    /// 群 UUID
    pub group_uuid: String,
    /// 序列化后的群消息体（QUIC 粘包协议格式）
    pub msg_bytes: Vec<u8>,
    /// 发送者 UUID
    pub sender: String,
    /// 全量群成员列表（各节点直接据此查本机 DashMap，无需再查 Redis）
    pub all_members: Vec<String>,
    /// 发送节点序号（用于跳过自身的广播）
    pub source_node: u32,
    /// 时间戳
    pub timestamp: i64,
    /// 全局唯一广播 ID（去重用）
    pub broadcast_id: String,
}

/// 群聊消息体（通过 QUIC 协议传输）
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupQuicMsg {
    pub nano_id: String,
    pub msg_type: u16,
    pub group_uuid: String,
    pub send_user: String,
    pub raw: Vec<u8>,
    pub timestamp: i64,
}

/// 群聊广播响应
#[derive(Debug, Serialize, Deserialize)]
pub struct InternalGroupBroadcastResponse {
    pub status: String,
    pub message: Option<String>,
}

impl InternalGroupBroadcastResponse {
    pub fn ok() -> Self {
        Self {
            status: "ok".to_string(),
            message: None,
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            status: "error".to_string(),
            message: Some(msg.into()),
        }
    }
}
