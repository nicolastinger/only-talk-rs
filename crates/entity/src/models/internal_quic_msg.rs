use serde::{Deserialize, Serialize};

/// 内网QUIC服务请求
#[derive(Debug, Serialize, Deserialize)]
pub struct InternalQuicRequest {
    /// 消息类型 (对应 message_types 中的常量)
    pub msg_type: u16,
    /// JSON 序列化的消息体
    pub payload: String,
    /// 目标用户UUID
    pub target_user: String,
}

/// 内网QUIC服务响应
#[derive(Debug, Serialize, Deserialize)]
pub struct InternalQuicResponse {
    /// "ok" 或 "error"
    pub status: String,
    /// 错误信息(仅 status="error" 时有值)
    pub message: Option<String>,
}

impl InternalQuicResponse {
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
