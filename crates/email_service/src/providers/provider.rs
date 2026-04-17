//! 邮件服务商 Trait 定义
//!
//! 本模块定义了 [`EmailProvider`] Trait，所有邮件服务商都需要实现此 Trait。

use async_trait::async_trait;
use std::sync::Arc;

use crate::config::ProviderConfig;
use crate::error::EmailResult;
use crate::models::{Email, SendResult};

/// 类型别名：装箱的 EmailProvider
///
/// 使用 `Arc<dyn EmailProvider + Send + Sync>` 实现动态分发，
/// 允许在运行时切换不同的服务商实现。
pub type BoxedEmailProvider = Arc<dyn EmailProvider + Send + Sync>;

/// 邮件服务商 Trait
///
/// 定义了邮件服务商必须实现的接口。所有服务商（阿里云、腾讯云、AWS SES、SMTP 等）
/// 都需要实现此 Trait。
///
/// # 核心方法
///
/// | 方法 | 必须实现 | 说明 |
/// |------|----------|------|
/// | [`name`](EmailProvider::name) | 是 | 服务商名称 |
/// | [`priority`](EmailProvider::priority) | 是 | 优先级 |
/// | [`is_enabled`](EmailProvider::is_enabled) | 是 | 是否启用 |
/// | [`health_check`](EmailProvider::health_check) | 是 | 健康检查 |
/// | [`send`](EmailProvider::send) | 是 | 发送邮件 |
///
/// # 能力声明方法
///
/// 以下方法用于声明服务商支持的功能，默认实现适用于大多数服务商：
///
/// - [`max_recipients_per_email`]: 单封邮件最大收件人数
/// - [`max_attachments_per_email`]: 单封邮件最大附件数
/// - [`max_attachment_size`]: 单个附件最大大小
/// - [`supports_html`]: 是否支持 HTML 正文
/// - [`supports_attachments`]: 是否支持附件
/// - [`supports_templates`]: 是否支持模板
/// - [`supports_scheduling`]: 是否支持定时发送
/// - [`supports_batch`]: 是否支持批量发送
///
/// # 示例
///
/// ```rust
/// use async_trait::async_trait;
/// use email_service::providers::{EmailProvider, BoxedEmailProvider};
/// use email_service::models::{Email, SendResult};
/// use email_service::error::EmailResult;
///
/// pub struct MyProvider {
///     config: MyConfig,
/// }
///
/// #[async_trait]
/// impl EmailProvider for MyProvider {
///     fn name(&self) -> &str {
///         "my_provider"
///     }
///
///     fn priority(&self) -> u32 {
///         100
///     }
///
///     fn is_enabled(&self) -> bool {
///         self.config.enabled
///     }
///
///     async fn health_check(&self) -> EmailResult<bool> {
///         // 检查服务商是否可用
///         Ok(true)
///     }
///
///     async fn send(&self, email: &Email) -> EmailResult<SendResult> {
///         // 实现发送逻辑
///         todo!()
///     }
/// }
/// # struct MyConfig { enabled: bool }
/// ```
#[async_trait]
pub trait EmailProvider: Send + Sync {
    /// 获取服务商名称
    ///
    /// 返回服务商的唯一标识符，用于日志和调试。
    fn name(&self) -> &str;

    /// 获取服务商优先级
    ///
    /// 数值越大优先级越高。当有多个服务商可用时，
    /// 会优先使用优先级高的服务商。
    fn priority(&self) -> u32;

    /// 检查服务商是否启用
    ///
    /// 禁用的服务商不会被用于发送邮件。
    fn is_enabled(&self) -> bool;

    /// 健康检查
    ///
    /// 检查服务商是否可用。返回 `Ok(true)` 表示健康，
    /// `Ok(false)` 表示不健康。
    async fn health_check(&self) -> EmailResult<bool>;

    /// 发送邮件
    ///
    /// # 参数
    ///
    /// - `email`: 要发送的邮件
    ///
    /// # 返回值
    ///
    /// 返回发送结果，即使发送失败也返回 `Ok(SendResult)`，
    /// 只有发生异常时才返回 `Err`。
    async fn send(&self, email: &Email) -> EmailResult<SendResult>;

    /// 批量发送邮件
    ///
    /// 默认实现是逐封发送。如果服务商支持批量 API，
    /// 可以覆盖此方法以提高效率。
    ///
    /// # 参数
    ///
    /// - `emails`: 要发送的邮件列表
    ///
    /// # 返回值
    ///
    /// 返回每封邮件的发送结果，顺序与输入一致。
    async fn send_batch(&self, emails: &[Email]) -> EmailResult<Vec<SendResult>> {
        let mut results = Vec::with_capacity(emails.len());
        for email in emails {
            results.push(self.send(email).await?);
        }
        Ok(results)
    }

    /// 单封邮件最大收件人数
    ///
    /// 默认值为 50。超过此限制会返回验证错误。
    fn max_recipients_per_email(&self) -> usize {
        50
    }

    /// 单封邮件最大附件数
    ///
    /// 默认值为 10。超过此限制会返回验证错误。
    fn max_attachments_per_email(&self) -> usize {
        10
    }

    /// 单个附件最大大小（字节）
    ///
    /// 默认值为 10MB。超过此限制会返回验证错误。
    fn max_attachment_size(&self) -> usize {
        10 * 1024 * 1024
    }

    /// 是否支持 HTML 正文
    ///
    /// 默认支持。如果不支持，发送 HTML 邮件会返回错误。
    fn supports_html(&self) -> bool {
        true
    }

    /// 是否支持附件
    ///
    /// 默认支持。如果不支持，发送带附件的邮件会返回错误。
    fn supports_attachments(&self) -> bool {
        true
    }

    /// 是否支持模板
    ///
    /// 默认不支持。如果支持，可以使用服务商的模板功能。
    fn supports_templates(&self) -> bool {
        false
    }

    /// 是否支持定时发送
    ///
    /// 默认不支持。如果支持，可以指定邮件发送时间。
    fn supports_scheduling(&self) -> bool {
        false
    }

    /// 是否支持批量发送
    ///
    /// 默认不支持。如果支持，可以使用服务商的批量 API。
    fn supports_batch(&self) -> bool {
        false
    }

    /// 验证邮件是否符合服务商限制
    ///
    /// 在发送前自动调用，检查邮件是否符合服务商的能力限制。
    ///
    /// # 检查项
    ///
    /// - 收件人数量
    /// - 附件数量
    /// - 附件大小
    /// - HTML 支持
    /// - 附件支持
    /// - 模板支持
    /// - 定时发送支持
    ///
    /// # 错误
    ///
    /// 如果验证失败，返回相应的错误。
    fn validate_email(&self, email: &Email) -> EmailResult<()> {
        if email.total_recipients() > self.max_recipients_per_email() {
            return Err(crate::error::EmailError::InvalidContent(format!(
                "Too many recipients: {} > {}",
                email.total_recipients(),
                self.max_recipients_per_email()
            )));
        }

        if email.attachments.len() > self.max_attachments_per_email() {
            return Err(crate::error::EmailError::InvalidContent(format!(
                "Too many attachments: {} > {}",
                email.attachments.len(),
                self.max_attachments_per_email()
            )));
        }

        for attachment in &email.attachments {
            if attachment.size() > self.max_attachment_size() {
                return Err(crate::error::EmailError::AttachmentError(format!(
                    "Attachment '{}' too large: {} bytes > {} bytes",
                    attachment.filename,
                    attachment.size(),
                    self.max_attachment_size()
                )));
            }
        }

        if email.html_body.is_some() && !self.supports_html() {
            return Err(crate::error::EmailError::InvalidContent(
                "HTML content not supported by this provider".to_string(),
            ));
        }

        if !email.attachments.is_empty() && !self.supports_attachments() {
            return Err(crate::error::EmailError::InvalidContent(
                "Attachments not supported by this provider".to_string(),
            ));
        }

        if email.template_id.is_some() && !self.supports_templates() {
            return Err(crate::error::EmailError::InvalidContent(
                "Templates not supported by this provider".to_string(),
            ));
        }

        if email.scheduled_at.is_some() && !self.supports_scheduling() {
            return Err(crate::error::EmailError::InvalidContent(
                "Scheduling not supported by this provider".to_string(),
            ));
        }

        email.validate()
    }
}

/// 邮件服务商工厂 Trait
///
/// 用于根据配置创建服务商实例。
///
/// # 示例
///
/// ```rust
/// use email_service::providers::{EmailProviderFactory, BoxedEmailProvider};
/// use email_service::config::ProviderConfig;
/// use email_service::error::EmailResult;
///
/// pub struct MyProviderFactory;
///
/// impl EmailProviderFactory for MyProviderFactory {
///     fn create(&self, config: &ProviderConfig) -> EmailResult<BoxedEmailProvider> {
///         // 根据配置创建服务商实例
///         todo!()
///     }
/// }
/// ```
pub trait EmailProviderFactory: Send + Sync {
    /// 创建服务商实例
    ///
    /// # 参数
    ///
    /// - `config`: 服务商配置
    ///
    /// # 返回值
    ///
    /// 返回装箱的服务商实例。
    fn create(&self, config: &ProviderConfig) -> EmailResult<BoxedEmailProvider>;
}
