use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use uuid::Uuid;

use super::{Attachment, EmailAddress};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Email {
    pub id: String,
    pub from: EmailAddress,
    pub to: Vec<EmailAddress>,
    pub cc: Vec<EmailAddress>,
    pub bcc: Vec<EmailAddress>,
    pub reply_to: Option<EmailAddress>,
    pub subject: String,
    pub text_body: Option<String>,
    pub html_body: Option<String>,
    pub attachments: Vec<Attachment>,
    pub headers: HashMap<String, String>,
    pub tags: HashMap<String, String>,
    pub priority: EmailPriority,
    pub created_at: DateTime<Utc>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub provider: Option<String>,
    pub template_id: Option<String>,
    pub template_data: HashMap<String, String>,
}

impl Email {
    pub fn builder() -> EmailBuilder {
        EmailBuilder::default()
    }

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

    pub fn with_text_body(mut self, body: impl Into<String>) -> Self {
        self.text_body = Some(body.into());
        self
    }

    pub fn with_html_body(mut self, body: impl Into<String>) -> Self {
        self.html_body = Some(body.into());
        self
    }

    pub fn with_cc(mut self, cc: Vec<EmailAddress>) -> Self {
        self.cc = cc;
        self
    }

    pub fn with_bcc(mut self, bcc: Vec<EmailAddress>) -> Self {
        self.bcc = bcc;
        self
    }

    pub fn with_reply_to(mut self, reply_to: EmailAddress) -> Self {
        self.reply_to = Some(reply_to);
        self
    }

    pub fn with_attachment(mut self, attachment: Attachment) -> Self {
        self.attachments.push(attachment);
        self
    }

    pub fn with_attachments(mut self, attachments: Vec<Attachment>) -> Self {
        self.attachments = attachments;
        self
    }

    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    pub fn with_tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }

    pub fn with_priority(mut self, priority: EmailPriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_provider(mut self, provider: impl Into<String>) -> Self {
        self.provider = Some(provider.into());
        self
    }

    pub fn with_template(mut self, template_id: impl Into<String>, data: HashMap<String, String>) -> Self {
        self.template_id = Some(template_id.into());
        self.template_data = data;
        self
    }

    pub fn schedule_at(mut self, time: DateTime<Utc>) -> Self {
        self.scheduled_at = Some(time);
        self
    }

    pub fn all_recipients(&self) -> Vec<&EmailAddress> {
        let mut recipients: Vec<&EmailAddress> = Vec::new();
        recipients.extend(self.to.iter());
        recipients.extend(self.cc.iter());
        recipients.extend(self.bcc.iter());
        recipients
    }

    pub fn total_recipients(&self) -> usize {
        self.to.len() + self.cc.len() + self.bcc.len()
    }

    pub fn has_attachments(&self) -> bool {
        !self.attachments.is_empty()
    }

    pub fn total_attachment_size(&self) -> usize {
        self.attachments.iter().map(|a| a.size()).sum()
    }

    pub fn is_html(&self) -> bool {
        self.html_body.is_some()
    }

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

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum EmailPriority {
    Low = 0,
    #[default]
    Normal = 1,
    High = 2,
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
    pub fn from(mut self, from: EmailAddress) -> Self {
        self.from = Some(from);
        self
    }

    pub fn to(mut self, to: EmailAddress) -> Self {
        self.to.push(to);
        self
    }

    pub fn to_multiple(mut self, recipients: Vec<EmailAddress>) -> Self {
        self.to.extend(recipients);
        self
    }

    pub fn cc(mut self, cc: EmailAddress) -> Self {
        self.cc.push(cc);
        self
    }

    pub fn bcc(mut self, bcc: EmailAddress) -> Self {
        self.bcc.push(bcc);
        self
    }

    pub fn reply_to(mut self, reply_to: EmailAddress) -> Self {
        self.reply_to = Some(reply_to);
        self
    }

    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    pub fn text_body(mut self, body: impl Into<String>) -> Self {
        self.text_body = Some(body.into());
        self
    }

    pub fn html_body(mut self, body: impl Into<String>) -> Self {
        self.html_body = Some(body.into());
        self
    }

    pub fn attachment(mut self, attachment: Attachment) -> Self {
        self.attachments.push(attachment);
        self
    }

    pub fn with_attachment(mut self, attachment: Attachment) -> Self {
        self.attachments.push(attachment);
        self
    }

    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    pub fn tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }

    pub fn priority(mut self, priority: EmailPriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn provider(mut self, provider: impl Into<String>) -> Self {
        self.provider = Some(provider.into());
        self
    }

    pub fn template(mut self, template_id: impl Into<String>, data: HashMap<String, String>) -> Self {
        self.template_id = Some(template_id.into());
        self.template_data = data;
        self
    }

    pub fn scheduled_at(mut self, time: DateTime<Utc>) -> Self {
        self.scheduled_at = Some(time);
        self
    }

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
