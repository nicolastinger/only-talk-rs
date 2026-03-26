use async_trait::async_trait;
use std::sync::Arc;

use crate::config::ProviderConfig;
use crate::error::EmailResult;
use crate::models::{Email, SendResult};

pub type BoxedEmailProvider = Arc<dyn EmailProvider + Send + Sync>;

#[async_trait]
pub trait EmailProvider: Send + Sync {
    fn name(&self) -> &str;

    fn priority(&self) -> u32;

    fn is_enabled(&self) -> bool;

    async fn health_check(&self) -> EmailResult<bool>;

    async fn send(&self, email: &Email) -> EmailResult<SendResult>;

    async fn send_batch(&self, emails: &[Email]) -> EmailResult<Vec<SendResult>> {
        let mut results = Vec::with_capacity(emails.len());
        for email in emails {
            results.push(self.send(email).await?);
        }
        Ok(results)
    }

    fn max_recipients_per_email(&self) -> usize {
        50
    }

    fn max_attachments_per_email(&self) -> usize {
        10
    }

    fn max_attachment_size(&self) -> usize {
        10 * 1024 * 1024
    }

    fn supports_html(&self) -> bool {
        true
    }

    fn supports_attachments(&self) -> bool {
        true
    }

    fn supports_templates(&self) -> bool {
        false
    }

    fn supports_scheduling(&self) -> bool {
        false
    }

    fn supports_batch(&self) -> bool {
        false
    }

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

pub trait EmailProviderFactory: Send + Sync {
    fn create(&self, config: &ProviderConfig) -> EmailResult<BoxedEmailProvider>;
}
