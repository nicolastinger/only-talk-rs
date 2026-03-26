use rand::Rng;
use std::time::Duration;
use tokio::time::sleep;

use crate::error::{EmailError, EmailResult};

#[derive(Debug, Clone)]
pub enum RetryStrategy {
    ExponentialBackoff(ExponentialBackoff),
    FixedInterval(FixedInterval),
    NoRetry(NoRetry),
    AdaptiveRetry(AdaptiveRetry),
}

impl RetryStrategy {
    pub async fn execute<F, Fut, T>(&self, operation: F) -> EmailResult<T>
    where
        F: Fn() -> Fut + Send,
        Fut: std::future::Future<Output = EmailResult<T>> + Send,
        T: Send,
    {
        match self {
            RetryStrategy::ExponentialBackoff(s) => s.execute(operation).await,
            RetryStrategy::FixedInterval(s) => s.execute(operation).await,
            RetryStrategy::NoRetry(s) => s.execute(operation).await,
            RetryStrategy::AdaptiveRetry(s) => s.execute(operation).await,
        }
    }

    pub fn exponential_backoff(max_attempts: u32) -> Self {
        RetryStrategy::ExponentialBackoff(ExponentialBackoff::new(max_attempts))
    }

    pub fn fixed_interval(max_attempts: u32, interval: Duration) -> Self {
        RetryStrategy::FixedInterval(FixedInterval::new(max_attempts, interval))
    }

    pub fn no_retry() -> Self {
        RetryStrategy::NoRetry(NoRetry)
    }

    pub fn adaptive(max_attempts: u32) -> Self {
        RetryStrategy::AdaptiveRetry(AdaptiveRetry::new(max_attempts))
    }

    pub fn from_config(config: &crate::config::RetryConfig) -> Self {
        RetryStrategy::ExponentialBackoff(ExponentialBackoff::from_config(config))
    }
}

impl Default for RetryStrategy {
    fn default() -> Self {
        RetryStrategy::exponential_backoff(3)
    }
}

#[derive(Debug, Clone)]
pub struct ExponentialBackoff {
    max_attempts: u32,
    initial_delay: Duration,
    max_delay: Duration,
    multiplier: f64,
    jitter: bool,
}

impl ExponentialBackoff {
    pub fn new(max_attempts: u32) -> Self {
        Self {
            max_attempts,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            multiplier: 2.0,
            jitter: true,
        }
    }

    pub fn with_initial_delay(mut self, delay: Duration) -> Self {
        self.initial_delay = delay;
        self
    }

    pub fn with_max_delay(mut self, delay: Duration) -> Self {
        self.max_delay = delay;
        self
    }

    pub fn with_multiplier(mut self, multiplier: f64) -> Self {
        self.multiplier = multiplier;
        self
    }

    pub fn with_jitter(mut self, jitter: bool) -> Self {
        self.jitter = jitter;
        self
    }

    pub fn from_config(config: &crate::config::RetryConfig) -> Self {
        Self {
            max_attempts: config.max_attempts,
            initial_delay: Duration::from_millis(config.initial_delay_ms),
            max_delay: Duration::from_millis(config.max_delay_ms),
            multiplier: config.multiplier,
            jitter: config.jitter,
        }
    }

    fn calculate_delay(&self, attempt: u32) -> Duration {
        let delay_ms = self.initial_delay.as_millis() as f64
            * self.multiplier.powi(attempt as i32 - 1);

        let delay = Duration::from_millis(delay_ms as u64).min(self.max_delay);

        if self.jitter {
            let jitter_range = delay.as_millis() as f64 * 0.3;
            let jitter = rand::thread_rng().gen_range(0.0..jitter_range) as u64;
            Duration::from_millis(delay.as_millis() as u64 + jitter)
        } else {
            delay
        }
    }

    pub async fn execute<F, Fut, T>(&self, operation: F) -> EmailResult<T>
    where
        F: Fn() -> Fut + Send,
        Fut: std::future::Future<Output = EmailResult<T>> + Send,
        T: Send,
    {
        let mut last_error: Option<EmailError> = None;

        for attempt in 1..=self.max_attempts {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(error) => {
                    if !error.is_retryable() {
                        return Err(error);
                    }

                    last_error = Some(error.clone());

                    if attempt < self.max_attempts {
                        let delay = self.calculate_delay(attempt);
                        tracing::warn!(
                            attempt = attempt,
                            max_attempts = self.max_attempts,
                            delay_ms = delay.as_millis(),
                            error = %error,
                            "Operation failed, retrying..."
                        );
                        sleep(delay).await;
                    }
                }
            }
        }

        Err(EmailError::RetryExhausted {
            attempts: self.max_attempts,
            last_error: last_error
                .map(|e| e.to_string())
                .unwrap_or_else(|| "Unknown error".to_string()),
        })
    }
}

#[derive(Debug, Clone)]
pub struct FixedInterval {
    max_attempts: u32,
    interval: Duration,
}

impl FixedInterval {
    pub fn new(max_attempts: u32, interval: Duration) -> Self {
        Self {
            max_attempts,
            interval,
        }
    }

    pub async fn execute<F, Fut, T>(&self, operation: F) -> EmailResult<T>
    where
        F: Fn() -> Fut + Send,
        Fut: std::future::Future<Output = EmailResult<T>> + Send,
        T: Send,
    {
        let mut last_error: Option<EmailError> = None;

        for attempt in 1..=self.max_attempts {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(error) => {
                    if !error.is_retryable() {
                        return Err(error);
                    }

                    last_error = Some(error.clone());

                    if attempt < self.max_attempts {
                        tracing::warn!(
                            attempt = attempt,
                            max_attempts = self.max_attempts,
                            interval_ms = self.interval.as_millis(),
                            error = %error,
                            "Operation failed, retrying..."
                        );
                        sleep(self.interval).await;
                    }
                }
            }
        }

        Err(EmailError::RetryExhausted {
            attempts: self.max_attempts,
            last_error: last_error
                .map(|e| e.to_string())
                .unwrap_or_else(|| "Unknown error".to_string()),
        })
    }
}

#[derive(Debug, Clone)]
pub struct NoRetry;

impl NoRetry {
    pub async fn execute<F, Fut, T>(&self, operation: F) -> EmailResult<T>
    where
        F: Fn() -> Fut + Send,
        Fut: std::future::Future<Output = EmailResult<T>> + Send,
        T: Send,
    {
        operation().await
    }
}

#[derive(Debug, Clone)]
pub struct AdaptiveRetry {
    base_strategy: ExponentialBackoff,
    circuit_breaker: CircuitBreaker,
}

impl AdaptiveRetry {
    pub fn new(max_attempts: u32) -> Self {
        Self {
            base_strategy: ExponentialBackoff::new(max_attempts),
            circuit_breaker: CircuitBreaker::new(5, Duration::from_secs(60)),
        }
    }

    pub async fn execute<F, Fut, T>(&self, operation: F) -> EmailResult<T>
    where
        F: Fn() -> Fut + Send,
        Fut: std::future::Future<Output = EmailResult<T>> + Send,
        T: Send,
    {
        self.circuit_breaker.check()?;

        let result = self.base_strategy.execute(operation).await;

        match &result {
            Ok(_) => {
                self.circuit_breaker.record_success();
            }
            Err(e) if e.is_retryable() => {
                self.circuit_breaker.record_failure();
            }
            _ => {}
        }

        result
    }
}

#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    failure_threshold: u32,
    failure_count: std::sync::Arc<parking_lot::Mutex<u32>>,
    reset_timeout: Duration,
    last_failure_time: std::sync::Arc<parking_lot::Mutex<Option<std::time::Instant>>>,
    state: std::sync::Arc<parking_lot::Mutex<CircuitState>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, reset_timeout: Duration) -> Self {
        Self {
            failure_threshold,
            failure_count: std::sync::Arc::new(parking_lot::Mutex::new(0)),
            reset_timeout,
            last_failure_time: std::sync::Arc::new(parking_lot::Mutex::new(None)),
            state: std::sync::Arc::new(parking_lot::Mutex::new(CircuitState::Closed)),
        }
    }

    pub fn check(&self) -> EmailResult<()> {
        let mut state = self.state.lock();

        match *state {
            CircuitState::Closed => Ok(()),
            CircuitState::Open => {
                let last_failure = self.last_failure_time.lock();
                if let Some(last_time) = *last_failure {
                    if last_time.elapsed() >= self.reset_timeout {
                        *state = CircuitState::HalfOpen;
                        Ok(())
                    } else {
                        Err(EmailError::ProviderUnavailable(
                            "Circuit breaker is open".to_string(),
                        ))
                    }
                } else {
                    *state = CircuitState::HalfOpen;
                    Ok(())
                }
            }
            CircuitState::HalfOpen => Ok(()),
        }
    }

    pub fn record_success(&self) {
        let mut state = self.state.lock();
        let mut count = self.failure_count.lock();

        *count = 0;
        *state = CircuitState::Closed;
    }

    pub fn record_failure(&self) {
        let mut count = self.failure_count.lock();
        let mut state = self.state.lock();
        let mut last_failure = self.last_failure_time.lock();

        *count += 1;
        *last_failure = Some(std::time::Instant::now());

        if *count >= self.failure_threshold {
            *state = CircuitState::Open;
        }
    }
}
