use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine};
use chrono::Utc;
use hmac::{Hmac, Mac};
use reqwest::Client;
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use std::sync::Arc;

use crate::config::TencentConfig;
use crate::error::{EmailError, EmailResult};
use crate::models::{Email, SendResult};
use crate::providers::{EmailProvider, BoxedEmailProvider};

type HmacSha256 = Hmac<Sha256>;

pub struct TencentEmailProvider {
    config: TencentConfig,
    client: Client,
    endpoint: String,
}

impl TencentEmailProvider {
    pub fn new(config: TencentConfig) -> EmailResult<Self> {
        let endpoint = format!(
            "ses.{}.tencentcloudapi.com",
            config.region
        );

        let client = Client::builder()
            .timeout(std::time::Duration::from_millis(config.timeout_ms))
            .build()
            .map_err(|e| EmailError::Config(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            config,
            client,
            endpoint,
        })
    }

    pub fn boxed(config: TencentConfig) -> EmailResult<BoxedEmailProvider> {
        Ok(Arc::new(Self::new(config)?))
    }

    fn generate_signature(&self, payload: &str, timestamp: i64) -> String {
        let date = Utc::now().format("%Y-%m-%d").to_string();
        let service = "ses";
        
        let string_to_sign = format!(
            "TC3-HMAC-SHA256\n{}\n{}/{}/tc3_request\n{}",
            timestamp,
            date,
            service,
            hex::encode(Sha256::digest(payload.as_bytes()))
        );

        let secret_date = Self::hmac_sha256(
            format!("TC3{}", self.config.secret_key).as_bytes(),
            date.as_bytes(),
        );
        let secret_service = Self::hmac_sha256(&secret_date, service.as_bytes());
        let secret_signing = Self::hmac_sha256(&secret_service, b"tc3_request");
        
        hex::encode(Self::hmac_sha256(&secret_signing, string_to_sign.as_bytes()))
    }

    fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
        let mut mac = HmacSha256::new_from_slice(key).expect("HMAC can take key of any size");
        mac.update(data);
        mac.finalize().into_bytes().to_vec()
    }

    fn build_headers(&self, payload: &str, action: &str) -> HashMap<String, String> {
        let timestamp = Utc::now().timestamp();
        let signature = self.generate_signature(payload, timestamp);
        let date = Utc::now().format("%Y-%m-%d").to_string();

        let mut headers = HashMap::new();
        headers.insert("Host".to_string(), self.endpoint.clone());
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("X-TC-Action".to_string(), action.to_string());
        headers.insert("X-TC-Version".to_string(), "2020-10-02".to_string());
        headers.insert("X-TC-Region".to_string(), self.config.region.clone());
        headers.insert("X-TC-Timestamp".to_string(), timestamp.to_string());
        headers.insert(
            "Authorization".to_string(),
            format!(
                "TC3-HMAC-SHA256 Credential={}/{}/ses/tc3_request, SignedHeaders=content-type;host, Signature={}",
                self.config.secret_id,
                date,
                signature
            ),
        );

        headers
    }

    async fn send_request<T: serde::Serialize>(&self, action: &str, body: T) -> EmailResult<serde_json::Value> {
        let payload = serde_json::to_string(&body)?;
        let headers = self.build_headers(&payload, action);

        let mut request = self.client
            .post(format!("https://{}", self.endpoint))
            .body(payload);

        for (key, value) in headers {
            request = request.header(key, value);
        }

        let response = request
            .send()
            .await
            .map_err(|e| EmailError::NetworkError(format!("Request failed: {}", e)))?;

        let status = response.status();
        let body = response.text().await
            .map_err(|e| EmailError::NetworkError(format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            tracing::error!("Tencent API error: status={}, body={}", status, body);
            return Err(EmailError::ProviderError {
                provider: "tencent".to_string(),
                message: format!("API error: {} - {}", status, body),
            });
        }

        serde_json::from_str(&body)
            .map_err(|e| EmailError::SerializationError(format!("Failed to parse response: {}", e)))
    }
}

#[derive(serde::Serialize)]
struct SendEmailRequest {
    from_email_address: String,
    destination: Destination,
    content: EmailContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    subject: Option<String>,
}

#[derive(serde::Serialize)]
struct Destination {
    to_addresses: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    cc_addresses: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    bcc_addresses: Vec<String>,
}

#[derive(serde::Serialize)]
struct EmailContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    simple: Option<SimpleContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    attachments: Option<Vec<Attachment>>,
}

#[derive(serde::Serialize)]
struct SimpleContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    html: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
}

#[derive(serde::Serialize)]
struct Attachment {
    content: String,
    filename: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content_type: Option<String>,
}

#[async_trait]
impl EmailProvider for TencentEmailProvider {
    fn name(&self) -> &str {
        "tencent"
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

        let from_email = if let Some(alias) = &self.config.from_alias {
            format!("{} <{}>", alias, self.config.sender)
        } else {
            self.config.sender.clone()
        };

        let destination = Destination {
            to_addresses: email.to.iter().map(|a| a.address().to_string()).collect(),
            cc_addresses: email.cc.iter().map(|a| a.address().to_string()).collect(),
            bcc_addresses: email.bcc.iter().map(|a| a.address().to_string()).collect(),
        };

        let simple = SimpleContent {
            html: email.html_body.clone(),
            text: email.text_body.clone(),
        };

        let attachments = if !email.attachments.is_empty() {
            Some(email.attachments.iter().map(|a| Attachment {
                content: BASE64_STANDARD.encode(&a.content),
                filename: a.filename.clone(),
                content_type: Some(a.content_type.clone()),
            }).collect())
        } else {
            None
        };

        let content = EmailContent {
            simple: Some(simple),
            attachments,
        };

        let request = SendEmailRequest {
            from_email_address: from_email,
            destination,
            content,
            subject: Some(email.subject.clone()),
        };

        match self.send_request("SendEmail", request).await {
            Ok(response) => {
                let message_id = response.get("Response")
                    .and_then(|r| r.get("MessageId"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let request_id = response.get("Response")
                    .and_then(|r| r.get("RequestId"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                Ok(SendResult::success("tencent", message_id, request_id))
            }
            Err(e) => {
                let error_info = crate::models::ErrorInfo::from_email_error(&e);
                Ok(SendResult::failure("tencent", error_info))
            }
        }
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
}
