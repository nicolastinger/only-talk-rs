use serde::{Deserialize, Serialize};

/// Request source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RequestSource {
    /// From external QUIC
    QuicExternal,
    /// From HTTP API
    HttpApi,
}

/// Internal QUIC service request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InternalQuicRequest {
    /// Message type (corresponds to constants in message_types)
    pub msg_type: u16,
    /// Serialized message body (bincode TextQuicMsg binary)
    pub payload: Vec<u8>,
    /// Target user UUID
    pub target_user: String,
    /// Preferred node index computed by hash modulo
    pub preferred_index: u32,
    /// Target platform PC / MOBILE
    pub platform: String,
    /// Request source
    pub source: RequestSource,
    /// Route TTL (decremented per hop, stops at 0)
    pub ttl: u8,
}

/// Internal QUIC service response
#[derive(Debug, Serialize, Deserialize)]
pub struct InternalQuicResponse {
    /// "ok" or "error"
    pub status: String,
    /// Error message (only present when status="error")
    pub message: Option<String>,
    /// Whether the message was delivered to the target client
    pub delivered: Option<bool>,
}

impl InternalQuicResponse {
    pub fn ok() -> Self {
        Self {
            status: "ok".to_string(),
            message: None,
            delivered: Some(true),
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            status: "error".to_string(),
            message: Some(msg.into()),
            delivered: None,
        }
    }

    pub fn user_offline() -> Self {
        Self {
            status: "ok".to_string(),
            message: Some("User offline".to_string()),
            delivered: Some(false),
        }
    }
}
