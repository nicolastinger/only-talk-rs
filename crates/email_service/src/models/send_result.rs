//! 邮件发送结果定义
//!
//! 本模块定义了邮件发送后的结果类型：
//!
//! - [`SendResult`]: 发送结果
//! - [`SendStatus`]: 发送状态
//! - [`RecipientResult`]: 收件人级别的结果
//! - [`ErrorInfo`]: 错误详情
//! - [`ErrorCategory`]: 错误分类

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 邮件发送结果
///
/// 表示邮件发送的完整结果，包含状态、消息 ID、错误信息等。
///
/// # 状态判断
///
/// - [`SendResult::is_success`]: 是否成功（包括部分成功）
/// - [`SendResult::is_queued`]: 是否在队列中
///
/// # 示例
///
/// ```rust
/// use email_service::models::{SendResult, SendStatus};
///
/// // 成功结果
/// let result = SendResult::success(
///     "aliyun",
///     Some("msg-12345".to_string()),
///     Some("req-67890".to_string())
/// );
///
/// assert!(result.is_success());
/// assert_eq!(result.status, SendStatus::Sent);
///
/// // 失败结果
/// use email_service::models::{ErrorInfo, ErrorCategory};
/// let error = ErrorInfo::new("RATE_LIMITED", "请求过于频繁", ErrorCategory::RateLimit);
/// let result = SendResult::failure("aliyun", error);
///
/// assert!(!result.is_success());
/// assert!(result.error.is_some());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendResult {
    /// 消息 ID
    ///
    /// 服务商返回的消息标识符，用于追踪邮件状态。
    pub message_id: Option<String>,

    /// 请求 ID
    ///
    /// 服务商返回的请求标识符，用于问题排查。
    pub request_id: Option<String>,

    /// 发送状态
    pub status: SendStatus,

    /// 服务商名称
    pub provider: String,

    /// 发送时间
    pub timestamp: DateTime<Utc>,

    /// 收件人级别的结果
    ///
    /// 群发邮件时，每个收件人可能有不同的结果。
    pub recipients: Vec<RecipientResult>,

    /// 错误信息（如果失败）
    pub error: Option<ErrorInfo>,

    /// 额外元数据
    pub metadata: serde_json::Value,
}

impl SendResult {
    /// 创建成功结果
    ///
    /// # 参数
    ///
    /// - `provider`: 服务商名称
    /// - `message_id`: 消息 ID（可选）
    /// - `request_id`: 请求 ID（可选）
    pub fn success(
        provider: impl Into<String>,
        message_id: Option<String>,
        request_id: Option<String>,
    ) -> Self {
        Self {
            message_id,
            request_id,
            status: SendStatus::Sent,
            provider: provider.into(),
            timestamp: Utc::now(),
            recipients: Vec::new(),
            error: None,
            metadata: serde_json::Value::Null,
        }
    }

    /// 创建失败结果
    ///
    /// # 参数
    ///
    /// - `provider`: 服务商名称
    /// - `error`: 错误信息
    pub fn failure(provider: impl Into<String>, error: ErrorInfo) -> Self {
        Self {
            message_id: None,
            request_id: None,
            status: SendStatus::Failed,
            provider: provider.into(),
            timestamp: Utc::now(),
            recipients: Vec::new(),
            error: Some(error),
            metadata: serde_json::Value::Null,
        }
    }

    /// 创建部分成功结果
    ///
    /// 群发邮件时，部分收件人成功，部分失败。
    ///
    /// # 参数
    ///
    /// - `provider`: 服务商名称
    /// - `message_id`: 消息 ID（可选）
    /// - `recipients`: 每个收件人的结果
    pub fn partial(
        provider: impl Into<String>,
        message_id: Option<String>,
        recipients: Vec<RecipientResult>,
    ) -> Self {
        let has_failures = recipients.iter().any(|r| !r.status.is_success());
        let status = if has_failures { SendStatus::PartialSuccess } else { SendStatus::Sent };

        Self {
            message_id,
            request_id: None,
            status,
            provider: provider.into(),
            timestamp: Utc::now(),
            recipients,
            error: None,
            metadata: serde_json::Value::Null,
        }
    }

    /// 添加收件人结果
    pub fn with_recipient(mut self, result: RecipientResult) -> Self {
        self.recipients.push(result);
        self
    }

    /// 添加元数据
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }

    /// 是否发送成功
    ///
    /// 包括完全成功和部分成功。
    pub fn is_success(&self) -> bool {
        matches!(self.status, SendStatus::Sent | SendStatus::PartialSuccess)
    }

    /// 是否在队列中
    ///
    /// 邮件已提交但尚未发送。
    pub fn is_queued(&self) -> bool {
        matches!(self.status, SendStatus::Queued | SendStatus::Scheduled)
    }
}

/// 发送状态
///
/// 表示邮件的当前状态，从提交到最终状态的完整生命周期。
///
/// # 状态流转
///
/// ```text
/// Queued -> Scheduled -> Sent -> Delivered -> Opened/Clicked
///                     \
///                      -> Bounced/Rejected/Failed
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SendStatus {
    /// 已入队
    ///
    /// 邮件已提交到发送队列，等待处理。
    Queued,

    /// 已计划
    ///
    /// 定时邮件已计划在指定时间发送。
    Scheduled,

    /// 已发送
    ///
    /// 邮件已成功发送到邮件服务器。
    Sent,

    /// 已送达
    ///
    /// 邮件已成功送达收件人邮箱。
    Delivered,

    /// 已打开
    ///
    /// 收件人已打开邮件（需要服务商支持追踪）。
    Opened,

    /// 已点击
    ///
    /// 收件人已点击邮件中的链接（需要服务商支持追踪）。
    Clicked,

    /// 已退信
    ///
    /// 邮件被退回，无法送达。
    Bounced,

    /// 已拒绝
    ///
    /// 邮件被服务商拒绝发送。
    Rejected,

    /// 发送失败
    ///
    /// 发送过程中发生错误。
    Failed,

    /// 部分成功
    ///
    /// 群发邮件时，部分收件人成功，部分失败。
    PartialSuccess,

    /// 已取消
    ///
    /// 邮件已取消发送。
    Cancelled,
}

impl SendStatus {
    /// 是否为成功状态
    ///
    /// 包括 Sent、Delivered、Opened、Clicked。
    pub fn is_success(&self) -> bool {
        matches!(
            self,
            SendStatus::Sent | SendStatus::Delivered | SendStatus::Opened | SendStatus::Clicked
        )
    }

    /// 是否为最终状态
    ///
    /// 最终状态不会再发生变化。
    pub fn is_final(&self) -> bool {
        matches!(
            self,
            SendStatus::Delivered
                | SendStatus::Bounced
                | SendStatus::Rejected
                | SendStatus::Failed
                | SendStatus::Cancelled
        )
    }
}

impl fmt::Display for SendStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SendStatus::Queued => write!(f, "queued"),
            SendStatus::Scheduled => write!(f, "scheduled"),
            SendStatus::Sent => write!(f, "sent"),
            SendStatus::Delivered => write!(f, "delivered"),
            SendStatus::Opened => write!(f, "opened"),
            SendStatus::Clicked => write!(f, "clicked"),
            SendStatus::Bounced => write!(f, "bounced"),
            SendStatus::Rejected => write!(f, "rejected"),
            SendStatus::Failed => write!(f, "failed"),
            SendStatus::PartialSuccess => write!(f, "partial_success"),
            SendStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// 收件人级别的发送结果
///
/// 群发邮件时，每个收件人可能有不同的发送状态。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipientResult {
    /// 收件人邮箱地址
    pub email: String,

    /// 发送状态
    pub status: SendStatus,

    /// 消息 ID（如果成功）
    pub message_id: Option<String>,

    /// 错误信息（如果失败）
    pub error: Option<ErrorInfo>,
}

impl RecipientResult {
    /// 创建成功结果
    pub fn success(email: impl Into<String>, message_id: Option<String>) -> Self {
        Self { email: email.into(), status: SendStatus::Sent, message_id, error: None }
    }

    /// 创建失败结果
    pub fn failure(email: impl Into<String>, error: ErrorInfo) -> Self {
        Self {
            email: email.into(),
            status: SendStatus::Failed,
            message_id: None,
            error: Some(error),
        }
    }
}

/// 错误详情
///
/// 包含错误的完整信息，便于调试和处理。
///
/// # 示例
///
/// ```rust
/// use email_service::models::{ErrorInfo, ErrorCategory};
///
/// let error = ErrorInfo::new(
///     "RATE_LIMITED",
///     "请求频率超过限制，请稍后重试",
///     ErrorCategory::RateLimit
/// ).with_retry(true, Some(60));
///
/// assert!(error.retryable);
/// assert_eq!(error.retry_after_seconds, Some(60));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorInfo {
    /// 错误代码
    ///
    /// 标准化的错误代码，便于程序处理。
    pub code: String,

    /// 错误信息
    ///
    /// 人类可读的错误描述。
    pub message: String,

    /// 错误分类
    pub category: ErrorCategory,

    /// 是否可重试
    pub retryable: bool,

    /// 建议等待时间（秒）
    pub retry_after_seconds: Option<u64>,
}

impl ErrorInfo {
    /// 创建错误信息
    ///
    /// # 参数
    ///
    /// - `code`: 错误代码
    /// - `message`: 错误描述
    /// - `category`: 错误分类
    pub fn new(
        code: impl Into<String>,
        message: impl Into<String>,
        category: ErrorCategory,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            category,
            retryable: false,
            retry_after_seconds: None,
        }
    }

    /// 设置重试信息
    ///
    /// # 参数
    ///
    /// - `retryable`: 是否可重试
    /// - `retry_after_seconds`: 建议等待时间
    pub fn with_retry(mut self, retryable: bool, retry_after_seconds: Option<u64>) -> Self {
        self.retryable = retryable;
        self.retry_after_seconds = retry_after_seconds;
        self
    }

    /// 从 EmailError 转换
    ///
    /// 将内部错误类型转换为可序列化的错误信息。
    pub fn from_email_error(error: &crate::error::EmailError) -> Self {
        let (code, category) = match error {
            crate::error::EmailError::Config(_) => ("CONFIG_ERROR", ErrorCategory::Configuration),
            crate::error::EmailError::AuthFailed { .. } => {
                ("AUTH_FAILED", ErrorCategory::Authentication)
            }
            crate::error::EmailError::RateLimited { .. } => {
                ("RATE_LIMITED", ErrorCategory::RateLimit)
            }
            crate::error::EmailError::InvalidEmailAddress(_) => {
                ("INVALID_EMAIL", ErrorCategory::Validation)
            }
            crate::error::EmailError::InvalidContent(_) => {
                ("INVALID_CONTENT", ErrorCategory::Validation)
            }
            crate::error::EmailError::AttachmentError(_) => {
                ("ATTACHMENT_ERROR", ErrorCategory::Validation)
            }
            crate::error::EmailError::ProviderError { .. } => {
                ("PROVIDER_ERROR", ErrorCategory::Provider)
            }
            crate::error::EmailError::NetworkError(_) => ("NETWORK_ERROR", ErrorCategory::Network),
            crate::error::EmailError::Timeout { .. } => ("TIMEOUT", ErrorCategory::Timeout),
            crate::error::EmailError::RetryExhausted { .. } => {
                ("RETRY_EXHAUSTED", ErrorCategory::System)
            }
            crate::error::EmailError::ProviderNotFound(_) => {
                ("PROVIDER_NOT_FOUND", ErrorCategory::Configuration)
            }
            crate::error::EmailError::ProviderUnavailable(_) => {
                ("PROVIDER_UNAVAILABLE", ErrorCategory::Provider)
            }
            crate::error::EmailError::TemplateError(_) => {
                ("TEMPLATE_ERROR", ErrorCategory::Template)
            }
            crate::error::EmailError::QueueError(_) => ("QUEUE_ERROR", ErrorCategory::System),
            crate::error::EmailError::SerializationError(_) => {
                ("SERIALIZATION_ERROR", ErrorCategory::System)
            }
            crate::error::EmailError::Unknown(_) => ("UNKNOWN", ErrorCategory::Unknown),
        };

        Self {
            code: code.to_string(),
            message: error.to_string(),
            category,
            retryable: error.is_retryable(),
            retry_after_seconds: None,
        }
    }
}

/// 错误分类
///
/// 将错误按类型分组，便于统计和处理。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    /// 配置错误
    Configuration,

    /// 认证错误
    Authentication,

    /// 限流错误
    RateLimit,

    /// 验证错误
    Validation,

    /// 服务商错误
    Provider,

    /// 网络错误
    Network,

    /// 超时错误
    Timeout,

    /// 模板错误
    Template,

    /// 系统错误
    System,

    /// 未知错误
    Unknown,
}

impl fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorCategory::Configuration => write!(f, "configuration"),
            ErrorCategory::Authentication => write!(f, "authentication"),
            ErrorCategory::RateLimit => write!(f, "rate_limit"),
            ErrorCategory::Validation => write!(f, "validation"),
            ErrorCategory::Provider => write!(f, "provider"),
            ErrorCategory::Network => write!(f, "network"),
            ErrorCategory::Timeout => write!(f, "timeout"),
            ErrorCategory::Template => write!(f, "template"),
            ErrorCategory::System => write!(f, "system"),
            ErrorCategory::Unknown => write!(f, "unknown"),
        }
    }
}

use std::fmt;
