use email_service::{EmailError, EmailResult};

#[test]
fn test_email_error_is_retryable() {
    let rate_limited = EmailError::RateLimited {
        provider: "test".to_string(),
        retry_after: None,
    };
    assert!(rate_limited.is_retryable());

    let network_error = EmailError::NetworkError("connection failed".to_string());
    assert!(network_error.is_retryable());

    let timeout = EmailError::Timeout {
        duration: std::time::Duration::from_secs(10),
        operation: "send".to_string(),
    };
    assert!(timeout.is_retryable());

    let provider_unavailable = EmailError::ProviderUnavailable("test".to_string());
    assert!(provider_unavailable.is_retryable());

    let auth_failed = EmailError::AuthFailed {
        provider: "test".to_string(),
        message: "invalid credentials".to_string(),
    };
    assert!(!auth_failed.is_retryable());

    let invalid_email = EmailError::InvalidEmailAddress("invalid".to_string());
    assert!(!invalid_email.is_retryable());
}

#[test]
fn test_email_error_provider_name() {
    let auth_failed = EmailError::AuthFailed {
        provider: "aliyun".to_string(),
        message: "test".to_string(),
    };
    assert_eq!(auth_failed.provider_name(), Some("aliyun"));

    let rate_limited = EmailError::RateLimited {
        provider: "tencent".to_string(),
        retry_after: None,
    };
    assert_eq!(rate_limited.provider_name(), Some("tencent"));

    let provider_error = EmailError::ProviderError {
        provider: "aws_ses".to_string(),
        message: "test".to_string(),
    };
    assert_eq!(provider_error.provider_name(), Some("aws_ses"));

    let network_error = EmailError::NetworkError("test".to_string());
    assert!(network_error.provider_name().is_none());
}

#[test]
fn test_email_error_display() {
    let config_error = EmailError::Config("missing key".to_string());
    assert!(config_error.to_string().contains("Configuration error"));

    let auth_error = EmailError::AuthFailed {
        provider: "test".to_string(),
        message: "invalid key".to_string(),
    };
    assert!(auth_error.to_string().contains("Authentication failed"));
    assert!(auth_error.to_string().contains("test"));

    let invalid_email = EmailError::InvalidEmailAddress("bad@email".to_string());
    assert!(invalid_email.to_string().contains("Invalid email address"));
}

#[test]
fn test_email_result() {
    fn returns_ok() -> EmailResult<String> {
        Ok("success".to_string())
    }

    fn returns_err() -> EmailResult<String> {
        Err(EmailError::Config("test error".to_string()))
    }

    assert!(returns_ok().is_ok());
    assert!(returns_err().is_err());
}
