//! # Email Service - 邮件服务库
//!
//! 一个功能强大、易于扩展的 Rust 邮件服务库，支持多种邮件服务提供商。
//!
//! ## 功能特性
//!
//! - **多服务商支持**: 支持阿里云、腾讯云、AWS SES、SMTP 等多种邮件服务商
//! - **智能重试机制**: 内置指数退避、固定间隔、自适应重试等策略
//! - **熔断器模式**: 自动检测服务商健康状态，防止级联故障
//! - **连接池管理**: 高效的连接复用和资源管理
//! - **速率限制**: 防止触发服务商的频率限制
//! - **故障转移**: 自动切换到备用服务商
//!
//! ## 快速开始
//!
//! ```rust,no_run
//! use email_service::prelude::*;
//! use email_service::{ProviderConfig, AliyunConfig, RetryConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // 创建邮件管理器
//!     let manager = EmailManager::builder()
//!         .default_provider("aliyun")
//!         .provider("aliyun", ProviderConfig::Aliyun(AliyunConfig {
//!             access_key_id: "your_access_key".to_string(),
//!             access_key_secret: "your_secret".to_string(),
//!             region_id: "cn-hangzhou".to_string(),
//!             account_name: "noreply@example.com".to_string(),
//!             ..Default::default()
//!         }))
//!         .retry_config(RetryConfig {
//!             max_attempts: 3,
//!             ..Default::default()
//!         })
//!         .build()?;
//!
//!     // 构建邮件
//!     let email = Email::builder()
//!         .from(EmailAddress::new("noreply@example.com")?)
//!         .to(EmailAddress::new("user@example.com")?)
//!         .subject("测试邮件")
//!         .text_body("这是一封测试邮件")
//!         .build()?;
//!
//!     // 发送邮件
//!     let result = manager.send(&email).await?;
//!     println!("发送结果: {:?}", result.status);
//!
//!     Ok(())
//! }
//! ```
//!
//! ## 架构设计
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                      EmailManager                        │
//! │  (统一入口，负责邮件发送、重试、故障转移)                    │
//! └─────────────────────────────────────────────────────────┘
//!                            │
//!                            ▼
//! ┌─────────────────────────────────────────────────────────┐
//! │                      ProviderPool                        │
//! │  (服务商池，管理多个服务商实例)                            │
//! └─────────────────────────────────────────────────────────┘
//!                            │
//!          ┌─────────────────┼─────────────────┐
//!          ▼                 ▼                 ▼
//!   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐
//!   │   Aliyun    │   │  Tencent    │   │   AWS SES   │
//!   │  Provider   │   │  Provider   │   │  Provider   │
//!   └─────────────┘   └─────────────┘   └─────────────┘
//! ```
//!
//! ## 模块说明
//!
//! - [`error`]: 错误类型定义
//! - [`config`]: 配置结构体
//! - [`models`]: 邮件模型（Email、Attachment 等）
//! - [`providers`]: 邮件服务商实现
//! - [`manager`]: 邮件管理器和连接池

pub mod config;
pub mod error;
pub mod manager;
pub mod models;
pub mod pool;
pub mod providers;

pub use config::{
    AliyunConfig, AwsSesConfig, EmailServiceConfig, PoolConfig, ProviderConfig, RateLimitConfig,
    RetryConfig, SmtpConfig, TencentConfig,
};
pub use error::{EmailError, EmailResult};
pub use manager::{
    EmailManager, EmailManagerBuilder, ProviderPool, ProviderSelector, SelectionStrategy,
};
pub use models::{
    Attachment, ContentDisposition, Email, EmailAddress, EmailBuilder, EmailPriority,
    ErrorCategory, ErrorInfo, RecipientResult, SendResult, SendStatus,
};
pub use providers::{
    AdaptiveRetry, AliyunEmailProvider, AwsSesEmailProvider, BoxedEmailProvider, CircuitBreaker,
    EmailProvider, ExponentialBackoff, FixedInterval, NoRetry, RetryStrategy, SmtpEmailProvider,
    TencentEmailProvider,
};

/// 常用类型预导入模块
///
/// 包含最常用的类型，方便快速上手：
///
/// ```rust
/// use email_service::prelude::*;
/// ```
pub mod prelude {
    pub use crate::config::{
        AliyunConfig, AwsSesConfig, EmailServiceConfig, PoolConfig, ProviderConfig, RetryConfig,
        SmtpConfig, TencentConfig,
    };
    pub use crate::error::{EmailError, EmailResult};
    pub use crate::manager::EmailManager;
    pub use crate::models::{
        Attachment, Email, EmailAddress, EmailBuilder, EmailPriority, SendResult, SendStatus,
    };
    pub use crate::providers::EmailProvider;
}
