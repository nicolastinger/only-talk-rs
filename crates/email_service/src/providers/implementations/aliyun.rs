use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine};
use chrono::Utc;
use hmac::{Hmac, Mac};
use reqwest::Client;
use sha2::Sha256;
use std::collections::HashMap;
use std::sync::Arc;

use crate::config::AliyunConfig;
use crate::error::{EmailError, EmailResult};
use crate::models::{Email, SendResult};
use crate::providers::{BoxedEmailProvider, EmailProvider};

type HmacSha256 = Hmac<Sha256>;

pub struct AliyunEmailProvider {
    config: AliyunConfig,
    client: Client,
    endpoint: String,
}

impl AliyunEmailProvider {
    pub fn new(config: AliyunConfig) -> EmailResult<Self> {
        let endpoint =
            config.endpoint.clone().unwrap_or_else(|| "https://dm.aliyuncs.com".to_string());

        let client = Client::builder()
            .timeout(std::time::Duration::from_millis(config.timeout_ms))
            .build()
            .map_err(|e| EmailError::Config(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { config, client, endpoint })
    }

    pub fn boxed(config: AliyunConfig) -> EmailResult<BoxedEmailProvider> {
        Ok(Arc::new(Self::new(config)?))
    }

    fn generate_signature(&self, params: &HashMap<String, String>) -> String {
        let mut sorted_params: Vec<_> = params.iter().collect();
        sorted_params.sort_by(|a, b| a.0.cmp(b.0));

        let canonicalized_query: String = sorted_params
            .iter()
            .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");

        let string_to_sign = format!("GET&%2F&{}", urlencoding::encode(&canonicalized_query));

        let key = format!("{}&", self.config.access_key_secret);
        let mut mac =
            HmacSha256::new_from_slice(key.as_bytes()).expect("HMAC can take key of any size");
        mac.update(string_to_sign.as_bytes());
        let result = mac.finalize();

        BASE64_STANDARD.encode(result.into_bytes())
    }

    fn build_common_params(&self) -> HashMap<String, String> {
        let mut params = HashMap::new();
        params.insert("Format".to_string(), "JSON".to_string());
        params.insert("Version".to_string(), "2015-11-23".to_string());
        params.insert("AccessKeyId".to_string(), self.config.access_key_id.clone());
        params.insert("SignatureMethod".to_string(), "HMAC-SHA256".to_string());
        params.insert("Timestamp".to_string(), Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string());
        params.insert("SignatureVersion".to_string(), "1.0".to_string());
        params.insert("SignatureNonce".to_string(), uuid::Uuid::new_v4().to_string());
        params
    }

    async fn send_request(
        &self,
        action: &str,
        params: HashMap<String, String>,
    ) -> EmailResult<serde_json::Value> {
        let mut all_params = self.build_common_params();
        all_params.insert("Action".to_string(), action.to_string());
        all_params.extend(params);

        let signature = self.generate_signature(&all_params);
        all_params.insert("Signature".to_string(), signature);

        let response = self
            .client
            .get(&self.endpoint)
            .query(&all_params)
            .send()
            .await
            .map_err(|e| EmailError::NetworkError(format!("Request failed: {}", e)))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| EmailError::NetworkError(format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            tracing::error!("Aliyun API error: status={}, body={}", status, body);
            return Err(EmailError::ProviderError {
                provider: "aliyun".to_string(),
                message: format!("API error: {} - {}", status, body),
            });
        }

        serde_json::from_str(&body)
            .map_err(|e| EmailError::SerializationError(format!("Failed to parse response: {}", e)))
    }
}

#[async_trait]
impl EmailProvider for AliyunEmailProvider {
    fn name(&self) -> &str {
        "aliyun"
    }

    fn priority(&self) -> u32 {
        self.config.priority
    }

    fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    async fn health_check(&self) -> EmailResult<bool> {
        let mut params = HashMap::new();
        params.insert("AccountName".to_string(), self.config.account_name.clone());

        match self.send_request("DescAccountSummary", params).await {
            Ok(_) => Ok(true),
            Err(e) => {
                tracing::warn!("Aliyun health check failed: {}", e);
                Ok(false)
            }
        }
    }

    async fn send(&self, email: &Email) -> EmailResult<SendResult> {
        self.validate_email(email)?;

        let mut params = HashMap::new();
        params.insert("AccountName".to_string(), self.config.account_name.clone());
        params.insert("AddressType".to_string(), "1".to_string());
        params.insert("ReplyToAddress".to_string(), "false".to_string());

        let from_alias =
            self.config.from_alias.clone().or_else(|| email.from.name().map(|s| s.to_string()));
        if let Some(alias) = from_alias {
            params.insert("FromAlias".to_string(), alias);
        }

        params.insert(
            "ToAddress".to_string(),
            email.to.iter().map(|a| a.address().to_string()).collect::<Vec<_>>().join(","),
        );

        if !email.cc.is_empty() {
            params.insert(
                "CcAddress".to_string(),
                email.cc.iter().map(|a| a.address().to_string()).collect::<Vec<_>>().join(","),
            );
        }

        if !email.bcc.is_empty() {
            params.insert(
                "BccAddress".to_string(),
                email.bcc.iter().map(|a| a.address().to_string()).collect::<Vec<_>>().join(","),
            );
        }

        params.insert("Subject".to_string(), email.subject.clone());

        if let Some(ref text) = email.text_body {
            params.insert("TextBody".to_string(), text.clone());
        }

        if let Some(ref html) = email.html_body {
            params.insert("HtmlBody".to_string(), html.clone());
        }

        if let Some(ref reply_to) = email.reply_to {
            params.insert("ReplyToAddress".to_string(), "true".to_string());
            params.insert("ReplyAddress".to_string(), reply_to.address().to_string());
        }

        for (i, attachment) in email.attachments.iter().enumerate() {
            let content_base64 = BASE64_STANDARD.encode(&attachment.content);
            params.insert(format!("Attachment{}.Name", i + 1), attachment.filename.clone());
            params.insert(format!("Attachment{}.Content", i + 1), content_base64);
        }

        match self.send_request("SingleSendMail", params).await {
            Ok(response) => {
                let env_id = response.get("EnvId").and_then(|v| v.as_str()).map(|s| s.to_string());

                Ok(SendResult::success("aliyun", env_id.clone(), env_id))
            }
            Err(e) => {
                let error_info = crate::models::ErrorInfo::from_email_error(&e);
                Ok(SendResult::failure("aliyun", error_info))
            }
        }
    }

    fn max_recipients_per_email(&self) -> usize {
        100
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

mod urlencoding {
    pub fn encode(s: &str) -> String {
        urlencoding::encode(s).replace('+', "%20").replace('*', "%2A")
    }
}
