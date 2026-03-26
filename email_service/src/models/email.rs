//! 邮件实体定义
//!
//! 本模块定义了 [`Email`] 结构体及其构建器 [`EmailBuilder`]。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use uuid::Uuid;

use super::{Attachment, EmailAddress};

/// 邮件实体
///
/// 表示一封完整的邮件，包含发件人、收件人、主题、正文、附件等信息。
///
/// # 字段说明
///
/// | 字段 | 必填 | 说明 |
/// |------|------|------|
/// | `from` | 是 | 发件人地址 |
/// | `to` | 是 | 收件人列表（至少一个） |
/// | `subject` | 是 | 邮件主题 |
/// | `text_body` / `html_body` | 二选一 | 纯文本或 HTML 正文 |
/// | `cc` | 否 | 抄送列表 |
/// | `bcc` | 否 | 密送列表 |
/// | `attachments` | 否 | 附件列表 |
///
/// # 构建方式
///
/// 推荐使用 [`Email::builder()`] 构建邮件：
///
/// ```rust
/// use email_service::models::{Email, EmailAddress};
///
/// let email = Email::builder()
///     .from(EmailAddress::new("noreply@example.com")?)
///     .to(EmailAddress::new("user@example.com")?)
///     .subject("测试邮件")
///     .text_body("这是邮件正文")
///     .build()?;
/// # Ok::<(), email_service::error::EmailError>(())
/// ```
///
/// # 邮件优先级
///
/// 通过 [`EmailPriority`] 设置邮件优先级：
///
/// - `Low`: 低优先级，适用于批量邮件
/// - `Normal`: 普通优先级（默认）
/// - `High`: 高优先级，适用于重要通知
/// - `Urgent`: 紧急优先级，适用于系统告警
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Email {
    /// 邮件唯一标识符
    ///
    /// 自动生成的 UUID，用于追踪邮件状态。
    pub id: String,

    /// 发件人地址
    pub from: EmailAddress,

    /// 收件人列表
    ///
    /// 至少需要一个收件人。
    pub to: Vec<EmailAddress>,

    /// 抄送列表
    ///
    /// 抄送的收件人会看到其他收件人。
    pub cc: Vec<EmailAddress>,

    /// 密送列表
    ///
    /// 密送的收件人不会看到其他收件人。
    pub bcc: Vec<EmailAddress>,

    /// 回复地址
    ///
    /// 如果设置，收件人回复时会发送到此地址。
    pub reply_to: Option<EmailAddress>,

    /// 邮件主题
    pub subject: String,

    /// 纯文本正文
    ///
    /// 适用于不支持 HTML 的邮件客户端。
    pub text_body: Option<String>,

    /// HTML 正文
    ///
    /// 支持富文本格式，可包含图片、链接等。
    pub html_body: Option<String>,

    /// 附件列表
    pub attachments: Vec<Attachment>,

    /// 自定义邮件头
    ///
    /// 可添加自定义的邮件头字段。
    pub headers: HashMap<String, String>,

    /// 标签
    ///
    /// 用于分类和追踪邮件，如 `{"type": "verification"}`。
    pub tags: HashMap<String, String>,

    /// 邮件优先级
    pub priority: EmailPriority,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 定时发送时间
    ///
    /// 如果设置，邮件将在指定时间发送。
    pub scheduled_at: Option<DateTime<Utc>>,

    /// 指定服务商
    ///
    /// 如果设置，将使用指定的服务商发送。
    pub provider: Option<String>,

    /// 模板 ID
    ///
    /// 如果使用模板发送，填写模板 ID。
    pub template_id: Option<String>,

    /// 模板变量
    ///
    /// 模板中变量的值，如 `{"name": "张三"}`。
    pub template_data: HashMap<String, String>,
}

impl Email {
    /// 创建邮件构建器
    ///
    /// 返回一个新的 [`EmailBuilder`] 实例。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use email_service::models::Email;
    ///
    /// let builder = Email::builder();
    /// ```
    pub fn builder() -> EmailBuilder {
        EmailBuilder::default()
    }

    /// 快速创建简单邮件
    ///
    /// 适用于只需要基本字段的场景。
    ///
    /// # 参数
    ///
    /// - `from`: 发件人地址
    /// - `to`: 收件人列表
    /// - `subject`: 邮件主题
    ///
    /// # 示例
    ///
    /// ```rust
    /// use email_service::models::{Email, EmailAddress};
    ///
    /// let email = Email::new(
    ///     EmailAddress::new("noreply@example.com")?,
    ///     vec![EmailAddress::new("user@example.com")?],
    ///     "测试邮件"
    /// );
    /// # Ok::<(), email_service::error::EmailError>(())
    /// ```
    pub fn new(
        from: EmailAddress,
        to: Vec<EmailAddress>,
        subject: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            from,
            to,
            cc: Vec::new(),
            bcc: Vec::new(),
            reply_to: None,
            subject: subject.into(),
            text_body: None,
            html_body: None,
            attachments: Vec::new(),
            headers: HashMap::new(),
            tags: HashMap::new(),
            priority: EmailPriority::Normal,
            created_at: Utc::now(),
            scheduled_at: None,
            provider: None,
            template_id: None,
            template_data: HashMap::new(),
        }
    }

    /// 设置纯文本正文
    pub fn with_text_body(mut self, body: impl Into<String>) -> Self {
        self.text_body = Some(body.into());
        self
    }

    /// 设置 HTML 正文
    pub fn with_html_body(mut self, body: impl Into<String>) -> Self {
        self.html_body = Some(body.into());
        self
    }

    /// 设置抄送列表
    pub fn with_cc(mut self, cc: Vec<EmailAddress>) -> Self {
        self.cc = cc;
        self
    }

    /// 设置密送列表
    pub fn with_bcc(mut self, bcc: Vec<EmailAddress>) -> Self {
        self.bcc = bcc;
        self
    }

    /// 设置回复地址
    pub fn with_reply_to(mut self, reply_to: EmailAddress) -> Self {
        self.reply_to = Some(reply_to);
        self
    }

    /// 添加附件
    pub fn with_attachment(mut self, attachment: Attachment) -> Self {
        self.attachments.push(attachment);
        self
    }

    /// 设置附件列表
    pub fn with_attachments(mut self, attachments: Vec<Attachment>) -> Self {
        self.attachments = attachments;
        self
    }

    /// 添加自定义邮件头
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// 添加标签
    pub fn with_tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }

    /// 设置优先级
    pub fn with_priority(mut self, priority: EmailPriority) -> Self {
        self.priority = priority;
        self
    }

    /// 指定服务商
    pub fn with_provider(mut self, provider: impl Into<String>) -> Self {
        self.provider = Some(provider.into());
        self
    }

    /// 设置模板
    pub fn with_template(mut self, template_id: impl Into<String>, data: HashMap<String, String>) -> Self {
        self.template_id = Some(template_id.into());
        self.template_data = data;
        self
    }

    /// 设置定时发送时间
    pub fn schedule_at(mut self, time: DateTime<Utc>) -> Self {
        self.scheduled_at = Some(time);
        self
    }

    /// 获取所有收件人
    ///
    /// 包括 `to`、`cc`、`bcc` 中的所有收件人。
    pub fn all_recipients(&self) -> Vec<&EmailAddress> {
        let mut recipients: Vec<&EmailAddress> = Vec::new();
        recipients.extend(self.to.iter());
        recipients.extend(self.cc.iter());
        recipients.extend(self.bcc.iter());
        recipients
    }

    /// 获取收件人总数
    pub fn total_recipients(&self) -> usize {
        self.to.len() + self.cc.len() + self.bcc.len()
    }

    /// 是否有附件
    pub fn has_attachments(&self) -> bool {
        !self.attachments.is_empty()
    }

    /// 获取附件总大小（字节）
    pub fn total_attachment_size(&self) -> usize {
        self.attachments.iter().map(|a| a.size()).sum()
    }

    /// 是否为 HTML 邮件
    pub fn is_html(&self) -> bool {
        self.html_body.is_some()
    }

    /// 验证邮件内容
    ///
    /// 检查必填字段是否完整。
    ///
    /// # 错误
    ///
    /// - 没有收件人
    /// - 主题为空
    /// - 没有正文（纯文本、HTML 或模板）
    pub fn validate(&self) -> crate::error::EmailResult<()> {
        if self.to.is_empty() && self.cc.is_empty() && self.bcc.is_empty() {
            return Err(crate::error::EmailError::InvalidContent(
                "No recipients specified".to_string(),
            ));
        }

        if self.subject.is_empty() {
            return Err(crate::error::EmailError::InvalidContent(
                "Subject cannot be empty".to_string(),
            ));
        }

        if self.text_body.is_none() && self.html_body.is_none() && self.template_id.is_none() {
            return Err(crate::error::EmailError::InvalidContent(
                "Email must have either text body, html body, or template".to_string(),
            ));
        }

        Ok(())
    }
}

/// 邮件优先级
///
/// 用于指示邮件的重要程度，部分服务商可能根据优先级调整发送顺序。
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum EmailPriority {
    /// 低优先级
    ///
    /// 适用于批量营销邮件、通知等。
    Low = 0,

    /// 普通优先级（默认）
    #[default]
    Normal = 1,

    /// 高优先级
    ///
    /// 适用于重要通知、验证码等。
    High = 2,

    /// 紧急优先级
    ///
    /// 适用于系统告警、紧急通知等。
    Urgent = 3,
}

impl fmt::Display for EmailPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmailPriority::Low => write!(f, "low"),
            EmailPriority::Normal => write!(f, "normal"),
            EmailPriority::High => write!(f, "high"),
            EmailPriority::Urgent => write!(f, "urgent"),
        }
    }
}

/// 邮件构建器
///
/// 使用 Builder 模式逐步构建邮件，确保必填字段完整。
///
/// # 示例
///
/// ```rust
/// use email_service::models::{Email, EmailAddress, EmailPriority};
///
/// let email = Email::builder()
///     .from(EmailAddress::new("noreply@example.com")?)
///     .to(EmailAddress::new("user@example.com")?)
///     .cc(EmailAddress::new("manager@example.com")?)
///     .subject("项目进度更新")
///     .html_body("<h1>本周进度</h1><p>项目进展顺利...</p>")
///     .text_body("本周进度\n项目进展顺利...")
///     .priority(EmailPriority::High)
///     .tag("department", "engineering")
///     .build()?;
/// # Ok::<(), email_service::error::EmailError>(())
/// ```
#[derive(Debug, Default)]
pub struct EmailBuilder {
    from: Option<EmailAddress>,
    to: Vec<EmailAddress>,
    cc: Vec<EmailAddress>,
    bcc: Vec<EmailAddress>,
    reply_to: Option<EmailAddress>,
    subject: Option<String>,
    text_body: Option<String>,
    html_body: Option<String>,
    attachments: Vec<Attachment>,
    headers: HashMap<String, String>,
    tags: HashMap<String, String>,
    priority: EmailPriority,
    provider: Option<String>,
    template_id: Option<String>,
    template_data: HashMap<String, String>,
    scheduled_at: Option<DateTime<Utc>>,
}

impl EmailBuilder {
    /// 设置发件人
    pub fn from(mut self, from: EmailAddress) -> Self {
        self.from = Some(from);
        self
    }

    /// 添加收件人
    pub fn to(mut self, to: EmailAddress) -> Self {
        self.to.push(to);
        self
    }

    /// 添加多个收件人
    pub fn to_multiple(mut self, recipients: Vec<EmailAddress>) -> Self {
        self.to.extend(recipients);
        self
    }

    /// 添加抄送
    pub fn cc(mut self, cc: EmailAddress) -> Self {
        self.cc.push(cc);
        self
    }

    /// 添加密送
    pub fn bcc(mut self, bcc: EmailAddress) -> Self {
        self.bcc.push(bcc);
        self
    }

    /// 设置回复地址
    pub fn reply_to(mut self, reply_to: EmailAddress) -> Self {
        self.reply_to = Some(reply_to);
        self
    }

    /// 设置主题
    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    /// 设置纯文本正文
    pub fn text_body(mut self, body: impl Into<String>) -> Self {
        self.text_body = Some(body.into());
        self
    }

    /// 设置 HTML 正文
    pub fn html_body(mut self, body: impl Into<String>) -> Self {
        self.html_body = Some(body.into());
        self
    }

    /// 添加附件
    pub fn attachment(mut self, attachment: Attachment) -> Self {
        self.attachments.push(attachment);
        self
    }

    /// 添加附件（别名）
    pub fn with_attachment(mut self, attachment: Attachment) -> Self {
        self.attachments.push(attachment);
        self
    }

    /// 添加自定义邮件头
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// 添加标签
    pub fn tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }

    /// 设置优先级
    pub fn priority(mut self, priority: EmailPriority) -> Self {
        self.priority = priority;
        self
    }

    /// 指定服务商
    pub fn provider(mut self, provider: impl Into<String>) -> Self {
        self.provider = Some(provider.into());
        self
    }

    /// 设置模板
    pub fn template(mut self, template_id: impl Into<String>, data: HashMap<String, String>) -> Self {
        self.template_id = Some(template_id.into());
        self.template_data = data;
        self
    }

    /// 设置定时发送时间
    pub fn scheduled_at(mut self, time: DateTime<Utc>) -> Self {
        self.scheduled_at = Some(time);
        self
    }

    /// 构建邮件
    ///
    /// 验证必填字段并返回 [`Email`] 实例。
    ///
    /// # 错误
    ///
    /// - 缺少发件人
    /// - 缺少主题
    /// - 没有收件人
    /// - 没有正文
    pub fn build(self) -> crate::error::EmailResult<Email> {
        let from = self.from.ok_or_else(|| {
            crate::error::EmailError::InvalidContent("From address is required".to_string())
        })?;

        let subject = self.subject.ok_or_else(|| {
            crate::error::EmailError::InvalidContent("Subject is required".to_string())
        })?;

        if self.to.is_empty() && self.cc.is_empty() && self.bcc.is_empty() {
            return Err(crate::error::EmailError::InvalidContent(
                "At least one recipient is required".to_string(),
            ));
        }

        if self.text_body.is_none() && self.html_body.is_none() && self.template_id.is_none() {
            return Err(crate::error::EmailError::InvalidContent(
                "Email must have either text body, html body, or template".to_string(),
            ));
        }

        Ok(Email {
            id: Uuid::new_v4().to_string(),
            from,
            to: self.to,
            cc: self.cc,
            bcc: self.bcc,
            reply_to: self.reply_to,
            subject,
            text_body: self.text_body,
            html_body: self.html_body,
            attachments: self.attachments,
            headers: self.headers,
            tags: self.tags,
            priority: self.priority,
            created_at: Utc::now(),
            scheduled_at: self.scheduled_at,
            provider: self.provider,
            template_id: self.template_id,
            template_data: self.template_data,
        })
    }
}
