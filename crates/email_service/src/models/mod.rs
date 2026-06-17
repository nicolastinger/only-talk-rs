//! 邮件模型定义
//!
//! 本模块定义了邮件相关的核心数据结构：
//!
//! - [`Email`]: 邮件实体，包含发件人、收件人、主题、正文等
//! - [`EmailAddress`]: 邮箱地址，支持显示名称
//! - [`Attachment`]: 邮件附件
//! - [`SendResult`]: 发送结果
//!
//! # 构建邮件
//!
//! 推荐使用 Builder 模式构建邮件：
//!
//! ```rust
//! use email_service::models::{Email, EmailAddress, EmailPriority};
//!
//! let email = Email::builder()
//!     .from(EmailAddress::new("noreply@example.com")?)
//!     .to(EmailAddress::with_name("user@example.com", "用户名")?)
//!     .subject("欢迎注册")
//!     .html_body("<h1>欢迎!</h1>")
//!     .priority(EmailPriority::High)
//!     .build()?;
//! # Ok::<(), email_service::error::EmailError>(())
//! ```

mod attachment;
mod email;
mod email_address;
mod send_result;

pub use attachment::*;
pub use email::*;
pub use email_address::*;
pub use send_result::*;
