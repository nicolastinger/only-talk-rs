use async_trait::async_trait;
use reqwest::Client;
use std::sync::Arc;

use crate::config::AwsSesConfig;
use crate::error::{EmailError, EmailResult};
use crate::models::{Email, SendResult};
use crate::providers::{BoxedEmailProvider, EmailProvider};

pub struct AwsSesEmailProvider {
    config: AwsSesConfig,
    client: Client,
}

impl AwsSesEmailProvider {
    pub fn new(config: AwsSesConfig) -> EmailResult<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_millis(config.timeout_ms))
            .build()
            .map_err(|e| EmailError::Config(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { config, client })
    }

    pub fn boxed(config: AwsSesConfig) -> EmailResult<BoxedEmailProvider> {
        Ok(Arc::new(Self::new(config)?))
    }

    async fn send_via_api(&self, email: &Email) -> EmailResult<SendResult> {
        let endpoint = format!(
            "https://email.{}.amazonaws.com/v2/email/outbound-emails",
            self.config.region
        );

        let to_addresses: Vec<String> = email.to.iter()
            .map(|a| a.address().to_string())
            .collect();
        
        let mut body = serde_json::json!({
            "FromEmailAddress": self.config.from_email,
            "Destination": {
                "ToAddresses": to_addresses,
            },
            "Content": {},
        });

        if let Some(ref alias) = self.config.from_alias {
            body["FromEmailAddress"] = serde_json::Value::String(
                format!("{} <{}>", alias, self.config.from_email)
            );
        }

        if !email.cc.is_empty() {
            let cc_addresses: Vec<String> = email.cc.iter()
                .map(|a| a.address().to_string())
                .collect();
            body["Destination"]["CcAddresses"] = serde_json::Value::Array(
                cc_addresses.into_iter().map(serde_json::Value::String).collect()
            );
        }

        if !email.bcc.is_empty() {
            let bcc_addresses: Vec<String> = email.bcc.iter()
                .map(|a| a.address().to_string())
                .collect();
            body["Destination"]["BccAddresses"] = serde_json::Value::Array(
                bcc_addresses.into_iter().map(serde_json::Value::String).collect()
            );
        }

        let mut simple = serde_json::json!({
            "Subject": {
                "Data": email.subject,
                "Charset": "UTF-8",
            },
        });

        if let Some(ref text) = email.text_body {
            simple["Body"]["Text"] = serde_json::json!({
                "Data": text,
                "Charset": "UTF-8",
            });
        }

        if let Some(ref html) = email.html_body {
            simple["Body"]["Html"] = serde_json::json!({
                "Data": html,
                "Charset": "UTF-8",
            });
        }

        body["Content"]["Simple"] = simple;

        let response = self.client
            .post(&endpoint)
            .json(&body)
            .basic_auth(&self.config.access_key_id, Some(&self.config.secret_access_key))
            .send()
            .await
            .map_err(|e| EmailError::NetworkError(format!("Request failed: {}", e)))?;

        if response.status().is_success() {
            let response_body = response.text().await.unwrap_or_default();
            let message_id = serde_json::from_str::<serde_json::Value>(&response_body)
                .ok()
                .and_then(|v| v.get("MessageId")?.as_str().map(|s| s.to_string()));
            
            Ok(SendResult::success("aws_ses", message_id, None))
        } else {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            let error_info = crate::models::ErrorInfo::from_email_error(
                &EmailError::ProviderError {
                    provider: "aws_ses".to_string(),
                    message: format!("API error: {} - {}", status, error_body),
                },
            );
            Ok(SendResult::failure("aws_ses", error_info))
        }
    }
}

#[async_trait]
impl EmailProvider for AwsSesEmailProvider {
    fn name(&self) -> &str {
        "aws_ses"
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
        true
    }
}
