use async_trait::async_trait;
use reqwest::Client;
use std::sync::Arc;

use crate::config::SmtpConfig;
use crate::error::{EmailError, EmailResult};
use crate::models::{Email, SendResult};
use crate::providers::{BoxedEmailProvider, EmailProvider};

pub struct SmtpEmailProvider {
    config: SmtpConfig,
    client: Client,
}

impl SmtpEmailProvider {
    pub fn new(config: SmtpConfig) -> EmailResult<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_millis(config.timeout_ms))
            .build()
            .map_err(|e| EmailError::Config(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { config, client })
    }

    pub fn boxed(config: SmtpConfig) -> EmailResult<BoxedEmailProvider> {
        Ok(Arc::new(Self::new(config)?))
    }

    async fn send_via_api(&self, email: &Email) -> EmailResult<SendResult> {
        let to_addresses: Vec<String> = email.to.iter().map(|a| a.address().to_string()).collect();

        let mut body = serde_json::json!({
            "from": {
                "email": self.config.from_email,
            },
            "to": to_addresses,
            "subject": email.subject,
        });

        if let Some(ref alias) = self.config.from_alias {
            body["from"]["name"] = serde_json::Value::String(alias.clone());
        }

        if let Some(ref text) = email.text_body {
            body["text"] = serde_json::Value::String(text.clone());
        }

        if let Some(ref html) = email.html_body {
            body["html"] = serde_json::Value::String(html.clone());
        }

        let response = self
            .client
            .post(&format!("https://{}:{}", self.config.host, self.config.port))
            .json(&body)
            .basic_auth(&self.config.username, Some(&self.config.password))
            .send()
            .await
            .map_err(|e| EmailError::NetworkError(format!("Request failed: {}", e)))?;

        if response.status().is_success() {
            Ok(SendResult::success("smtp", None, None))
        } else {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            let error_info =
                crate::models::ErrorInfo::from_email_error(&EmailError::ProviderError {
                    provider: "smtp".to_string(),
                    message: format!("API error: {} - {}", status, error_body),
                });
            Ok(SendResult::failure("smtp", error_info))
        }
    }
}

#[async_trait]
impl EmailProvider for SmtpEmailProvider {
    fn name(&self) -> &str {
        "smtp"
    }

    fn priority(&self) -> u32 {
        self.config.priority
    }

    fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    async fn health_check(&self) -> EmailResult<bool> {
        Ok(true)
    }

    async fn send(&self, email: &Email) -> EmailResult<SendResult> {
        self.validate_email(email)?;
        self.send_via_api(email).await
    }

    fn max_recipients_per_email(&self) -> usize {
        100
    }

    fn max_attachments_per_email(&self) -> usize {
        20
    }

    fn max_attachment_size(&self) -> usize {
        25 * 1024 * 1024
    }

    fn supports_html(&self) -> bool {
        true
    }

    fn supports_attachments(&self) -> bool {
        true
    }
}
