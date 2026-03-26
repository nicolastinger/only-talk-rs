//! 邮件服务错误类型定义
//!
//! 本模块定义了邮件服务中所有可能的错误类型，使用 [`thiserror`] 库实现。
//!
//! # 错误分类
//!
//! 错误分为以下几类：
//!
//! | 类别 | 错误类型 | 是否可重试 |
//! |------|----------|------------|
//! | 配置错误 | [`Config`] | 否 |
//! | 认证错误 | [`AuthFailed`] | 否 |
//! | 限流错误 | [`RateLimited`] | 是 |
//! | 验证错误 | [`InvalidEmailAddress`], [`InvalidContent`], [`AttachmentError`] | 否 |
//! | 服务商错误 | [`ProviderError`], [`ProviderNotFound`], [`ProviderUnavailable`] | 部分 |
//! | 网络错误 | [`NetworkError`], [`Timeout`] | 是 |
//! | 系统错误 | [`RetryExhausted`], [`QueueError`], [`SerializationError`] | 否 |
//!
//! # 示例
//!
//! ```rust
//! use email_service::error::{EmailError, EmailResult};
//!
//! fn send_email() -> EmailResult<()> {
//!     Err(EmailError::ProviderError {
//!         provider: "aliyun".to_string(),
//!         message: "服务暂时不可用".to_string(),
//!     })
//! }
//!
//! match send_email() {
//!     Ok(_) => println!("发送成功"),
//!     Err(e) if e.is_retryable() => println!("可重试错误: {}", e),
//!     Err(e) => println!("不可重试错误: {}", e),
//! }
//! ```

use std::time::Duration;
use thiserror::Error;

/// 邮件服务错误枚举
///
/// 定义了邮件发送过程中可能遇到的所有错误类型。
/// 每种错误都包含足够的上下文信息，便于调试和处理。
#[derive(Error, Debug, Clone)]
pub enum EmailError {
    /// 配置错误
    ///
    /// 当配置项缺失、格式错误或无效时返回。
    ///
    /// # 示例
    /// ```rust
    /// use email_service::error::EmailError;
    ///
    /// let error = EmailError::Config("缺少 access_key_id".to_string());
    /// ```
    #[error("Configuration error: {0}")]
    Config(String),

    /// 认证失败
    ///
    /// 当服务商拒绝认证时返回，通常是由于密钥无效或已过期。
    ///
    /// # 字段
    /// - `provider`: 服务商名称
    /// - `message`: 错误详情
    #[error("Authentication failed for provider '{provider}': {message}")]
    AuthFailed {
        /// 服务商名称
        provider: String,
        /// 错误详情
        message: String,
    },

    /// 请求被限流
    ///
    /// 当服务商返回限流错误时返回，通常需要等待一段时间后重试。
    ///
    /// # 字段
    /// - `provider`: 服务商名称
    /// - `retry_after`: 建议等待时间（如果服务商提供）
    #[error("Rate limited by provider '{provider}', retry after {retry_after:?}")]
    RateLimited {
        /// 服务商名称
        provider: String,
        /// 建议等待时间
        retry_after: Option<Duration>,
    },

    /// 无效的邮箱地址
    ///
    /// 当邮箱地址格式不正确时返回。
    #[error("Invalid email address: {0}")]
    InvalidEmailAddress(String),

    /// 无效的邮件内容
    ///
    /// 当邮件内容不符合要求时返回（如缺少主题、正文等）。
    #[error("Invalid email content: {0}")]
    InvalidContent(String),

    /// 附件错误
    ///
    /// 当附件处理失败时返回（如文件过大、格式不支持等）。
    #[error("Attachment error: {0}")]
    AttachmentError(String),

    /// 服务商错误
    ///
    /// 当服务商返回错误时返回，包含服务商名称和错误信息。
    #[error("Provider '{provider}' error: {message}")]
    ProviderError {
        /// 服务商名称
        provider: String,
        /// 错误信息
        message: String,
    },

    /// 网络错误
    ///
    /// 当网络请求失败时返回（如 DNS 解析失败、连接超时等）。
    #[error("Network error: {0}")]
    NetworkError(String),

    /// 超时错误
    ///
    /// 当操作超时时返回。
    #[error("Timeout error after {duration:?}: {operation}")]
    Timeout {
        /// 超时时长
        duration: Duration,
        /// 超时的操作名称
        operation: String,
    },

    /// 重试次数耗尽
    ///
    /// 当所有重试尝试都失败时返回。
    #[error("Retry exhausted after {attempts} attempts: {last_error}")]
    RetryExhausted {
        /// 尝试次数
        attempts: u32,
        /// 最后一次错误信息
        last_error: String,
    },

    /// 服务商未找到
    ///
    /// 当请求的服务商不存在时返回。
    #[error("Provider '{0}' not found")]
    ProviderNotFound(String),

    /// 服务商不可用
    ///
    /// 当服务商处于熔断状态或无法连接时返回。
    #[error("Provider '{0}' is unavailable")]
    ProviderUnavailable(String),

    /// 模板错误
    ///
    /// 当邮件模板处理失败时返回。
    #[error("Template error: {0}")]
    TemplateError(String),

    /// 队列错误
    ///
    /// 当邮件队列操作失败时返回。
    #[error("Queue error: {0}")]
    QueueError(String),

    /// 序列化错误
    ///
    /// 当 JSON 序列化/反序列化失败时返回。
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// 未知错误
    ///
    /// 当发生无法分类的错误时返回。
    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl EmailError {
    /// 判断错误是否可重试
    ///
    /// 某些错误（如网络错误、限流错误）可以通过重试来解决，
    /// 而配置错误、认证错误等则需要人工干预。
    ///
    /// # 返回值
    ///
    /// - `true`: 错误可重试，建议等待后重新发送
    /// - `false`: 错误不可重试，需要人工干预
    ///
    /// # 示例
    ///
    /// ```rust
    /// use email_service::error::EmailError;
    /// use std::time::Duration;
    ///
    /// let error = EmailError::RateLimited {
    ///     provider: "aliyun".to_string(),
    ///     retry_after: Some(Duration::from_secs(60)),
    /// };
    ///
    /// if error.is_retryable() {
    ///     println!("等待后重试...");
    /// }
    /// ```
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            EmailError::RateLimited { .. }
                | EmailError::NetworkError(_)
                | EmailError::Timeout { .. }
                | EmailError::ProviderUnavailable(_)
        )
    }

    /// 获取错误关联的服务商名称
    ///
    /// 某些错误与服务商相关，此方法可以提取服务商名称。
    ///
    /// # 返回值
    ///
    /// - `Some(&str)`: 错误关联的服务商名称
    /// - `None`: 错误与服务商无关
    ///
    /// # 示例
    ///
    /// ```rust
    /// use email_service::error::EmailError;
    ///
    /// let error = EmailError::ProviderError {
    ///     provider: "aliyun".to_string(),
    ///     message: "服务不可用".to_string(),
    /// };
    ///
    /// if let Some(provider) = error.provider_name() {
    ///     println!("服务商 {} 出错", provider);
    /// }
    /// ```
    pub fn provider_name(&self) -> Option<&str> {
        match self {
            EmailError::AuthFailed { provider, .. } => Some(provider),
            EmailError::RateLimited { provider, .. } => Some(provider),
            EmailError::ProviderError { provider, .. } => Some(provider),
            EmailError::ProviderNotFound(provider) => Some(provider),
            EmailError::ProviderUnavailable(provider) => Some(provider),
            _ => None,
        }
    }
}

/// 邮件服务结果类型别名
///
/// 使用 `Result<T, EmailError>` 的简写形式，方便函数签名。
///
/// # 示例
///
/// ```rust
/// use email_service::error::{EmailError, EmailResult};
///
/// fn send_email() -> EmailResult<String> {
///     Ok("message_id_123".to_string())
/// }
/// ```
pub type EmailResult<T> = Result<T, EmailError>;

impl From<serde_json::Error> for EmailError {
    fn from(err: serde_json::Error) -> Self {
        EmailError::SerializationError(err.to_string())
    }
}

impl From<std::io::Error> for EmailError {
    fn from(err: std::io::Error) -> Self {
        EmailError::NetworkError(err.to_string())
    }
}
