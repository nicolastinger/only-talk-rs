//! 邮件服务商模块
//!
//! 本模块定义了邮件服务商的核心抽象和实现：
//!
//! - [`EmailProvider`]: 服务商 Trait，定义统一接口
//! - [`RetryStrategy`]: 重试策略
//! - [`CircuitBreaker`]: 熔断器
//!
//! # 支持的服务商
//!
//! | 服务商 | 模块 | 说明 |
//! |--------|------|------|
//! | 阿里云 | [`AliyunEmailProvider`] | 阿里云邮件推送 |
//! | 腾讯云 | [`TencentEmailProvider`] | 腾讯云 SES |
//! | AWS | [`AwsSesEmailProvider`] | Amazon SES |
//! | SMTP | [`SmtpEmailProvider`] | 通用 SMTP 服务器 |
//!
//! # 扩展服务商
//!
//! 实现 [`EmailProvider`] Trait 即可添加新的服务商：
//!
//! ```rust
//! use async_trait::async_trait;
//! use email_service::providers::{EmailProvider, BoxedEmailProvider};
//! use email_service::models::{Email, SendResult};
//! use email_service::error::EmailResult;
//!
//! pub struct MyEmailProvider {
//!     // 配置字段
//! }
//!
//! #[async_trait]
//! impl EmailProvider for MyEmailProvider {
//!     fn name(&self) -> &str {
//!         "my_provider"
//!     }
//!
//!     fn priority(&self) -> u32 {
//!         100
//!     }
//!
//!     fn is_enabled(&self) -> bool {
//!         true
//!     }
//!
//!     async fn health_check(&self) -> EmailResult<bool> {
//!         // 实现健康检查
//!         Ok(true)
//!     }
//!
//!     async fn send(&self, email: &Email) -> EmailResult<SendResult> {
//!         // 实现邮件发送逻辑
//!         todo!()
//!     }
//! }
//! ```

mod provider;
mod retry_strategy;
pub mod implementations;

pub use provider::*;
pub use retry_strategy::*;
pub use implementations::*;
