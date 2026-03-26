use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum EmailError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Authentication failed for provider '{provider}': {message}")]
    AuthFailed { provider: String, message: String },

    #[error("Rate limited by provider '{provider}', retry after {retry_after:?}")]
    RateLimited {
        provider: String,
        retry_after: Option<Duration>,
    },

    #[error("Invalid email address: {0}")]
    InvalidEmailAddress(String),

    #[error("Invalid email content: {0}")]
    InvalidContent(String),

    #[error("Attachment error: {0}")]
    AttachmentError(String),

    #[error("Provider '{provider}' error: {message}")]
    ProviderError { provider: String, message: String },

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Timeout error after {duration:?}: {operation}")]
    Timeout { duration: Duration, operation: String },

    #[error("Retry exhausted after {attempts} attempts: {last_error}")]
    RetryExhausted { attempts: u32, last_error: String },

    #[error("Provider '{0}' not found")]
    ProviderNotFound(String),

    #[error("Provider '{0}' is unavailable")]
    ProviderUnavailable(String),

    #[error("Template error: {0}")]
    TemplateError(String),

    #[error("Queue error: {0}")]
    QueueError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl EmailError {
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            EmailError::RateLimited { .. }
                | EmailError::NetworkError(_)
                | EmailError::Timeout { .. }
                | EmailError::ProviderUnavailable(_)
        )
    }

    pub fn provider_name(&self) -> Option<&str> {
        match self {
            EmailError::AuthFailed { provider, .. } => Some(provider),
            EmailError::RateLimited { provider, .. } => Some(provider),
            EmailError::ProviderError { provider, .. } => Some(provider),
            EmailError::ProviderNotFound(provider) => Some(provider),
            EmailError::ProviderUnavailable(provider) => Some(provider),
            _ => None,
        }
    }
}

pub type EmailResult<T> = Result<T, EmailError>;

impl From<serde_json::Error> for EmailError {
    fn from(err: serde_json::Error) -> Self {
        EmailError::SerializationError(err.to_string())
    }
}

impl From<std::io::Error> for EmailError {
    fn from(err: std::io::Error) -> Self {
        EmailError::NetworkError(err.to_string())
    }
}
