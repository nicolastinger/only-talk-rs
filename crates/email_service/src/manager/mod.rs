//! 邮件管理器模块
//!
//! 本模块提供邮件发送的核心管理功能：
//!
//! - [`EmailManager`]: 邮件管理器，统一管理服务商和发送逻辑
//! - [`EmailManagerBuilder`]: 管理器构建器
//! - [`ProviderPool`]: 服务商连接池
//! - [`ProviderSelector`]: 服务商选择器
//!
//! # 快速开始
//!
//! ```rust
//! use email_service::manager::EmailManager;
//! use email_service::{ProviderConfig, AliyunConfig};
//!
//! let manager = EmailManager::builder()
//!     .default_provider("aliyun")
//!     .provider("aliyun", ProviderConfig::Aliyun(AliyunConfig {
//!         access_key_id: "your_key".to_string(),
//!         access_key_secret: "your_secret".to_string(),
//!         region_id: "cn-hangzhou".to_string(),
//!         account_name: "noreply@example.com".to_string(),
//!         ..Default::default()
//!     }))
//!     .build()?;
//! # Ok::<(), email_service::error::EmailError>(())
//! ```

mod email_manager;
pub mod provider_pool;

pub use email_manager::*;
pub use provider_pool::*;
