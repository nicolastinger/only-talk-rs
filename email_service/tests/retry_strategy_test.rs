use email_service::{RetryStrategy, RetryConfig};
use email_service::error::EmailError;
use std::time::Duration;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

#[tokio::test]
async fn test_retry_strategy_exponential_backoff() {
    let strategy = RetryStrategy::exponential_backoff(3);
    
    let attempts = Arc::new(AtomicU32::new(0));
    let attempts_clone = attempts.clone();
    
    let result = strategy.execute(move || {
        let attempts = attempts_clone.clone();
        async move {
            let count = attempts.fetch_add(1, Ordering::SeqCst) + 1;
            if count < 3 {
                Err(EmailError::NetworkError("temporary failure".to_string()))
            } else {
                Ok("success")
            }
        }
    }).await;

    assert!(result.is_ok());
    assert_eq!(attempts.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn test_retry_strategy_no_retry_on_non_retryable() {
    let strategy = RetryStrategy::exponential_backoff(3);
    
    let attempts = Arc::new(AtomicU32::new(0));
    let attempts_clone = attempts.clone();
    
    let result: Result<String, _> = strategy.execute(move || {
        let attempts = attempts_clone.clone();
        async move {
            attempts.fetch_add(1, Ordering::SeqCst);
            Err(EmailError::InvalidEmailAddress("invalid".to_string()))
        }
    }).await;

    assert!(result.is_err());
    assert_eq!(attempts.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_retry_strategy_fixed_interval() {
    let strategy = RetryStrategy::fixed_interval(2, Duration::from_millis(10));
    
    let attempts = Arc::new(AtomicU32::new(0));
    let attempts_clone = attempts.clone();
    
    let result = strategy.execute(move || {
        let attempts = attempts_clone.clone();
        async move {
            let count = attempts.fetch_add(1, Ordering::SeqCst) + 1;
            if count < 2 {
                Err(EmailError::NetworkError("fail".to_string()))
            } else {
                Ok("success")
            }
        }
    }).await;

    assert!(result.is_ok());
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn test_retry_strategy_no_retry() {
    let strategy = RetryStrategy::no_retry();
    
    let attempts = Arc::new(AtomicU32::new(0));
    let attempts_clone = attempts.clone();
    
    let result: Result<String, _> = strategy.execute(move || {
        let attempts = attempts_clone.clone();
        async move {
            attempts.fetch_add(1, Ordering::SeqCst);
            Err(EmailError::NetworkError("fail".to_string()))
        }
    }).await;

    assert!(result.is_err());
    assert_eq!(attempts.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_retry_strategy_exhausted() {
    let strategy = RetryStrategy::exponential_backoff(2);
    
    let attempts = Arc::new(AtomicU32::new(0));
    let attempts_clone = attempts.clone();
    
    let result: Result<String, _> = strategy.execute(move || {
        let attempts = attempts_clone.clone();
        async move {
            attempts.fetch_add(1, Ordering::SeqCst);
            Err(EmailError::NetworkError("always fail".to_string()))
        }
    }).await;

    assert!(result.is_err());
    if let Err(EmailError::RetryExhausted { attempts: count, .. }) = result {
        assert_eq!(count, 2);
    } else {
        panic!("Expected RetryExhausted error");
    }
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
}

#[test]
fn test_retry_strategy_from_config() {
    let config = RetryConfig {
        max_attempts: 5,
        initial_delay_ms: 200,
        max_delay_ms: 60000,
        multiplier: 2.5,
        jitter: false,
    };

    let _strategy = RetryStrategy::from_config(&config);
}

#[test]
fn test_retry_strategy_default() {
    let _strategy = RetryStrategy::default();
}

#[test]
fn test_retry_strategy_helpers() {
    let _strategy = RetryStrategy::exponential_backoff(5);
    let _strategy = RetryStrategy::fixed_interval(3, Duration::from_secs(1));
    let _strategy = RetryStrategy::no_retry();
    let _strategy = RetryStrategy::adaptive(3);
}

#[tokio::test]
async fn test_retry_success_immediately() {
    let strategy = RetryStrategy::exponential_backoff(3);
    
    let attempts = Arc::new(AtomicU32::new(0));
    let attempts_clone = attempts.clone();
    
    let result = strategy.execute(move || {
        let attempts = attempts_clone.clone();
        async move {
            attempts.fetch_add(1, Ordering::SeqCst);
            Ok("immediate success")
        }
    }).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "immediate success");
    assert_eq!(attempts.load(Ordering::SeqCst), 1);
}
