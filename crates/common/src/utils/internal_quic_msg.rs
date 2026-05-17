use serde::{Deserialize, Serialize};

/// 请求来源
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RequestSource {
    /// 来自外网 QUIC
    QuicExternal,
    /// 来自 HTTP API
    HttpApi,
}

/// 内网QUIC服务请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InternalQuicRequest {
    /// 消息类型 (对应 message_types 中的常量)
    pub msg_type: u16,
    /// JSON 序列化的消息体
    pub payload: String,
    /// 目标用户UUID
    pub target_user: String,
    /// hash 取模得到的首选节点序号
    pub preferred_index: u32,
    /// 目标平台 PC / MOBILE
    pub platform: String,
    /// 请求来源
    pub source: RequestSource,
    /// 路由跳数 (每跳一次减 1，0 时停止)
    pub ttl: u8,
}

/// 内网QUIC服务响应
#[derive(Debug, Serialize, Deserialize)]
pub struct InternalQuicResponse {
    /// "ok" 或 "error"
    pub status: String,
    /// 错误信息(仅 status="error" 时有值)
    pub message: Option<String>,
    /// 消息是否已投递到目标客户端
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
            message: Some("用户不在线".to_string()),
            delivered: Some(false),
        }
    }
}
