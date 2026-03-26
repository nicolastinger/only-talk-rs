use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendResult {
    pub message_id: Option<String>,
    pub request_id: Option<String>,
    pub status: SendStatus,
    pub provider: String,
    pub timestamp: DateTime<Utc>,
    pub recipients: Vec<RecipientResult>,
    pub error: Option<ErrorInfo>,
    pub metadata: serde_json::Value,
}

impl SendResult {
    pub fn success(
        provider: impl Into<String>,
        message_id: Option<String>,
        request_id: Option<String>,
    ) -> Self {
        Self {
            message_id,
            request_id,
            status: SendStatus::Sent,
            provider: provider.into(),
            timestamp: Utc::now(),
            recipients: Vec::new(),
            error: None,
            metadata: serde_json::Value::Null,
        }
    }

    pub fn failure(
        provider: impl Into<String>,
        error: ErrorInfo,
    ) -> Self {
        Self {
            message_id: None,
            request_id: None,
            status: SendStatus::Failed,
            provider: provider.into(),
            timestamp: Utc::now(),
            recipients: Vec::new(),
            error: Some(error),
            metadata: serde_json::Value::Null,
        }
    }

    pub fn partial(
        provider: impl Into<String>,
        message_id: Option<String>,
        recipients: Vec<RecipientResult>,
    ) -> Self {
        let has_failures = recipients.iter().any(|r| !r.status.is_success());
        let status = if has_failures {
            SendStatus::PartialSuccess
        } else {
            SendStatus::Sent
        };

        Self {
            message_id,
            request_id: None,
            status,
            provider: provider.into(),
            timestamp: Utc::now(),
            recipients,
            error: None,
            metadata: serde_json::Value::Null,
        }
    }

    pub fn with_recipient(mut self, result: RecipientResult) -> Self {
        self.recipients.push(result);
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn is_success(&self) -> bool {
        matches!(self.status, SendStatus::Sent | SendStatus::PartialSuccess)
    }

    pub fn is_queued(&self) -> bool {
        matches!(self.status, SendStatus::Queued | SendStatus::Scheduled)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SendStatus {
    Queued,
    Scheduled,
    Sent,
    Delivered,
    Opened,
    Clicked,
    Bounced,
    Rejected,
    Failed,
    PartialSuccess,
    Cancelled,
}

impl SendStatus {
    pub fn is_success(&self) -> bool {
        matches!(
            self,
            SendStatus::Sent
                | SendStatus::Delivered
                | SendStatus::Opened
                | SendStatus::Clicked
        )
    }

    pub fn is_final(&self) -> bool {
        matches!(
            self,
            SendStatus::Delivered
                | SendStatus::Bounced
                | SendStatus::Rejected
                | SendStatus::Failed
                | SendStatus::Cancelled
        )
    }
}

impl fmt::Display for SendStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SendStatus::Queued => write!(f, "queued"),
            SendStatus::Scheduled => write!(f, "scheduled"),
            SendStatus::Sent => write!(f, "sent"),
            SendStatus::Delivered => write!(f, "delivered"),
            SendStatus::Opened => write!(f, "opened"),
            SendStatus::Clicked => write!(f, "clicked"),
            SendStatus::Bounced => write!(f, "bounced"),
            SendStatus::Rejected => write!(f, "rejected"),
            SendStatus::Failed => write!(f, "failed"),
            SendStatus::PartialSuccess => write!(f, "partial_success"),
            SendStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipientResult {
    pub email: String,
    pub status: SendStatus,
    pub message_id: Option<String>,
    pub error: Option<ErrorInfo>,
}

impl RecipientResult {
    pub fn success(email: impl Into<String>, message_id: Option<String>) -> Self {
        Self {
            email: email.into(),
            status: SendStatus::Sent,
            message_id,
            error: None,
        }
    }

    pub fn failure(email: impl Into<String>, error: ErrorInfo) -> Self {
        Self {
            email: email.into(),
            status: SendStatus::Failed,
            message_id: None,
            error: Some(error),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorInfo {
    pub code: String,
    pub message: String,
    pub category: ErrorCategory,
    pub retryable: bool,
    pub retry_after_seconds: Option<u64>,
}

impl ErrorInfo {
    pub fn new(code: impl Into<String>, message: impl Into<String>, category: ErrorCategory) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            category,
            retryable: false,
            retry_after_seconds: None,
        }
    }

    pub fn with_retry(mut self, retryable: bool, retry_after_seconds: Option<u64>) -> Self {
        self.retryable = retryable;
        self.retry_after_seconds = retry_after_seconds;
        self
    }

    pub fn from_email_error(error: &crate::error::EmailError) -> Self {
        let (code, category) = match error {
            crate::error::EmailError::Config(_) => ("CONFIG_ERROR", ErrorCategory::Configuration),
            crate::error::EmailError::AuthFailed { .. } => ("AUTH_FAILED", ErrorCategory::Authentication),
            crate::error::EmailError::RateLimited { .. } => ("RATE_LIMITED", ErrorCategory::RateLimit),
            crate::error::EmailError::InvalidEmailAddress(_) => ("INVALID_EMAIL", ErrorCategory::Validation),
            crate::error::EmailError::InvalidContent(_) => ("INVALID_CONTENT", ErrorCategory::Validation),
            crate::error::EmailError::AttachmentError(_) => ("ATTACHMENT_ERROR", ErrorCategory::Validation),
            crate::error::EmailError::ProviderError { .. } => ("PROVIDER_ERROR", ErrorCategory::Provider),
            crate::error::EmailError::NetworkError(_) => ("NETWORK_ERROR", ErrorCategory::Network),
            crate::error::EmailError::Timeout { .. } => ("TIMEOUT", ErrorCategory::Timeout),
            crate::error::EmailError::RetryExhausted { .. } => ("RETRY_EXHAUSTED", ErrorCategory::System),
            crate::error::EmailError::ProviderNotFound(_) => ("PROVIDER_NOT_FOUND", ErrorCategory::Configuration),
            crate::error::EmailError::ProviderUnavailable(_) => ("PROVIDER_UNAVAILABLE", ErrorCategory::Provider),
            crate::error::EmailError::TemplateError(_) => ("TEMPLATE_ERROR", ErrorCategory::Template),
            crate::error::EmailError::QueueError(_) => ("QUEUE_ERROR", ErrorCategory::System),
            crate::error::EmailError::SerializationError(_) => ("SERIALIZATION_ERROR", ErrorCategory::System),
            crate::error::EmailError::Unknown(_) => ("UNKNOWN", ErrorCategory::Unknown),
        };

        Self {
            code: code.to_string(),
            message: error.to_string(),
            category,
            retryable: error.is_retryable(),
            retry_after_seconds: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    Configuration,
    Authentication,
    RateLimit,
    Validation,
    Provider,
    Network,
    Timeout,
    Template,
    System,
    Unknown,
}

impl fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorCategory::Configuration => write!(f, "configuration"),
            ErrorCategory::Authentication => write!(f, "authentication"),
            ErrorCategory::RateLimit => write!(f, "rate_limit"),
            ErrorCategory::Validation => write!(f, "validation"),
            ErrorCategory::Provider => write!(f, "provider"),
            ErrorCategory::Network => write!(f, "network"),
            ErrorCategory::Timeout => write!(f, "timeout"),
            ErrorCategory::Template => write!(f, "template"),
            ErrorCategory::System => write!(f, "system"),
            ErrorCategory::Unknown => write!(f, "unknown"),
        }
    }
}

use std::fmt;
