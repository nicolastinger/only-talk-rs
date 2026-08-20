use serde::{Deserialize, Serialize};

/// 请求来源
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RequestSource {
    /// 来自外部 QUIC
    QuicExternal,
    /// 来自 HTTP API
    HttpApi,
}

/// 内部 QUIC 服务请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InternalQuicRequest {
    /// 消息类型(对应 message_types 中的常量)
    pub msg_type: u16,
    /// 序列化后的消息体(bincode TextQuicMsg 二进制)
    pub payload: Vec<u8>,
    /// 目标用户 UUID
    pub target_user: String,
    /// 通过哈希取模计算的首选节点索引
    pub preferred_index: u32,
    /// 目标平台 PC / MOBILE
    pub platform: String,
    /// 请求来源
    pub source: RequestSource,
    /// 路由 TTL(每跳减一,到 0 停止)
    pub ttl: u8,
}

/// 内部 QUIC 服务响应
#[derive(Debug, Serialize, Deserialize)]
pub struct InternalQuicResponse {
    /// "ok" 或 "error"
    pub status: String,
    /// 错误信息(仅当 status="error" 时存在)
    pub message: Option<String>,
    /// 消息是否已投递到目标客户端
    pub delivered: Option<bool>,
}

impl InternalQuicResponse {
    pub fn ok() -> Self {
        Self { status: "ok".to_string(), message: None, delivered: Some(true) }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self { status: "error".to_string(), message: Some(msg.into()), delivered: None }
    }

    pub fn user_offline() -> Self {
        Self {
            status: "ok".to_string(),
            message: Some("User offline".to_string()),
            delivered: Some(false),
        }
    }
}
