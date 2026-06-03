use serde::{Deserialize, Serialize};

/// Broadcast type
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

/// Internal QUIC group message broadcast request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InternalGroupBroadcast {
    /// Message type identifier
    pub broadcast_type: BroadcastType,
    /// Group UUID
    pub group_uuid: String,
    /// Serialized group message body (QUIC sticky packet protocol format)
    pub msg_bytes: Vec<u8>,
    /// Sender UUID
    pub sender: String,
    /// Full group member list (each node queries local DashMap directly, no Redis needed)
    pub all_members: Vec<String>,
    /// Sending node index (used to skip self in broadcast)
    pub source_node: u32,
    /// Timestamp
    pub timestamp: i64,
    /// Globally unique broadcast ID (for deduplication)
    pub broadcast_id: String,
}

/// Group chat message body (transmitted via QUIC protocol)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupQuicMsg {
    pub nano_id: String,
    pub msg_type: u16,
    pub group_uuid: String,
    pub send_user: String,
    pub raw: Vec<u8>,
    pub timestamp: i64,
}

/// Group chat broadcast response
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
