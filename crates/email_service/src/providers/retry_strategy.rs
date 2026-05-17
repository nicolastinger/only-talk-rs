//! 重试策略和熔断器实现
//!
//! 本模块提供了多种重试策略和熔断器实现：
//!
//! - [`RetryStrategy`]: 重试策略枚举
//! - [`ExponentialBackoff`]: 指数退避策略
//! - [`FixedInterval`]: 固定间隔策略
//! - [`NoRetry`]: 不重试策略
//! - [`AdaptiveRetry`]: 自适应重试策略（带熔断器）
//! - [`CircuitBreaker`]: 熔断器
//!
//! # 重试策略选择
//!
//! | 策略 | 适用场景 | 特点 |
//! |------|----------|------|
//! | `ExponentialBackoff` | 通用场景 | 延迟逐渐增加，避免雪崩 |
//! | `FixedInterval` | 简单场景 | 固定延迟，易于预测 |
//! | `NoRetry` | 关键操作 | 失败立即返回 |
//! | `AdaptiveRetry` | 生产环境 | 带熔断保护，自动降级 |
//!
//! # 示例
//!
//! ```rust
//! use email_service::providers::{RetryStrategy, CircuitBreaker};
//! use std::time::Duration;
//!
//! // 指数退避
//! let strategy = RetryStrategy::exponential_backoff(3);
//!
//! // 固定间隔
//! let strategy = RetryStrategy::fixed_interval(3, Duration::from_secs(1));
//!
//! // 不重试
//! let strategy = RetryStrategy::no_retry();
//!
//! // 自适应重试（带熔断器）
//! let strategy = RetryStrategy::adaptive(5);
//! ```

use rand::Rng;
use std::time::Duration;
use tokio::time::sleep;

use crate::error::{EmailError, EmailResult};

/// 重试策略枚举
///
/// 封装了所有重试策略，提供统一的接口。
///
/// # 示例
///
/// ```rust
/// use email_service::providers::RetryStrategy;
/// use email_service::error::EmailResult;
///
/// async fn send_with_retry() -> EmailResult<String> {
///     let strategy = RetryStrategy::exponential_backoff(3);
///
///     strategy.execute(|| async {
///         // 模拟发送操作
///         Ok("success".to_string())
///     }).await
/// }
/// ```
#[derive(Debug, Clone)]
pub enum RetryStrategy {
    /// 指数退避策略
    ///
    /// 每次重试的延迟时间按指数增长，适合大多数场景。
    ExponentialBackoff(ExponentialBackoff),

    /// 固定间隔策略
    ///
    /// 每次重试的延迟时间固定，适合简单场景。
    FixedInterval(FixedInterval),

    /// 不重试策略
    ///
    /// 失败立即返回，适合关键操作。
    NoRetry(NoRetry),

    /// 自适应重试策略
    ///
    /// 结合熔断器，在连续失败后自动熔断。
    AdaptiveRetry(AdaptiveRetry),
}

impl RetryStrategy {
    /// 执行带重试的操作
    ///
    /// 根据策略自动重试失败的操作。
    ///
    /// # 参数
    ///
    /// - `operation`: 要执行的操作（返回 `EmailResult<T>` 的异步闭包）
    ///
    /// # 返回值
    ///
    /// - `Ok(T)`: 操作成功
    /// - `Err(EmailError)`: 所有重试都失败
    ///
    /// # 示例
    ///
    /// ```rust
    /// use email_service::providers::RetryStrategy;
    /// use email_service::error::{EmailError, EmailResult};
    ///
    /// async fn send_email() -> EmailResult<()> {
    ///     let strategy = RetryStrategy::exponential_backoff(3);
    ///
    ///     strategy.execute(|| async {
    ///         // 模拟可能失败的操作
    ///         if rand::random::<bool>() {
    ///             Ok(())
    ///         } else {
    ///             Err(EmailError::NetworkError("连接失败".to_string()))
    ///         }
    ///     }).await
    /// }
    /// ```
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

    /// 创建指数退避策略
    ///
    /// # 参数
    ///
    /// - `max_attempts`: 最大尝试次数
    pub fn exponential_backoff(max_attempts: u32) -> Self {
        RetryStrategy::ExponentialBackoff(ExponentialBackoff::new(max_attempts))
    }

    /// 创建固定间隔策略
    ///
    /// # 参数
    ///
    /// - `max_attempts`: 最大尝试次数
    /// - `interval`: 重试间隔
    pub fn fixed_interval(max_attempts: u32, interval: Duration) -> Self {
        RetryStrategy::FixedInterval(FixedInterval::new(max_attempts, interval))
    }

    /// 创建不重试策略
    pub fn no_retry() -> Self {
        RetryStrategy::NoRetry(NoRetry)
    }

    /// 创建自适应重试策略
    ///
    /// # 参数
    ///
    /// - `max_attempts`: 最大尝试次数
    pub fn adaptive(max_attempts: u32) -> Self {
        RetryStrategy::AdaptiveRetry(AdaptiveRetry::new(max_attempts))
    }

    /// 从配置创建重试策略
    ///
    /// 使用 [`RetryConfig`] 创建指数退避策略。
    pub fn from_config(config: &crate::config::RetryConfig) -> Self {
        RetryStrategy::ExponentialBackoff(ExponentialBackoff::from_config(config))
    }
}

impl Default for RetryStrategy {
    fn default() -> Self {
        RetryStrategy::exponential_backoff(3)
    }
}

/// 指数退避重试策略
///
/// 每次重试的延迟时间按指数增长，是最常用的重试策略。
///
/// # 延迟计算公式
///
/// ```text
/// delay = min(initial_delay * multiplier^(attempt-1), max_delay)
/// ```
///
/// 如果启用抖动（jitter），会在计算结果上添加随机偏移。
///
/// # 示例
///
/// ```rust
/// use email_service::providers::ExponentialBackoff;
/// use std::time::Duration;
///
/// let strategy = ExponentialBackoff::new(5)
///     .with_initial_delay(Duration::from_millis(100))
///     .with_max_delay(Duration::from_secs(30))
///     .with_multiplier(2.0)
///     .with_jitter(true);
/// ```
///
/// # 重试时间线示例
///
/// | 尝试 | 基础延迟 | 含抖动（约） |
/// |------|----------|--------------|
/// | 1 | 100ms | 100-130ms |
/// | 2 | 200ms | 200-260ms |
/// | 3 | 400ms | 400-520ms |
/// | 4 | 800ms | 800-1040ms |
/// | 5 | 1600ms | 1600-2080ms |
#[derive(Debug, Clone)]
pub struct ExponentialBackoff {
    max_attempts: u32,
    initial_delay: Duration,
    max_delay: Duration,
    multiplier: f64,
    jitter: bool,
}

impl ExponentialBackoff {
    /// 创建指数退避策略
    ///
    /// # 参数
    ///
    /// - `max_attempts`: 最大尝试次数
    ///
    /// 默认配置：
    /// - 初始延迟: 100ms
    /// - 最大延迟: 30s
    /// - 乘数: 2.0
    /// - 抖动: 启用
    pub fn new(max_attempts: u32) -> Self {
        Self {
            max_attempts,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            multiplier: 2.0,
            jitter: true,
        }
    }

    /// 设置初始延迟
    pub fn with_initial_delay(mut self, delay: Duration) -> Self {
        self.initial_delay = delay;
        self
    }

    /// 设置最大延迟
    pub fn with_max_delay(mut self, delay: Duration) -> Self {
        self.max_delay = delay;
        self
    }

    /// 设置延迟乘数
    pub fn with_multiplier(mut self, multiplier: f64) -> Self {
        self.multiplier = multiplier;
        self
    }

    /// 设置是否启用抖动
    pub fn with_jitter(mut self, jitter: bool) -> Self {
        self.jitter = jitter;
        self
    }

    /// 从配置创建
    pub fn from_config(config: &crate::config::RetryConfig) -> Self {
        Self {
            max_attempts: config.max_attempts,
            initial_delay: Duration::from_millis(config.initial_delay_ms),
            max_delay: Duration::from_millis(config.max_delay_ms),
            multiplier: config.multiplier,
            jitter: config.jitter,
        }
    }

    /// 计算第 N 次重试的延迟时间
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

    /// 执行带重试的操作
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

/// 固定间隔重试策略
///
/// 每次重试的延迟时间固定，适合简单场景或需要精确控制的场景。
///
/// # 示例
///
/// ```rust
/// use email_service::providers::FixedInterval;
/// use std::time::Duration;
///
/// let strategy = FixedInterval::new(3, Duration::from_secs(2));
/// ```
#[derive(Debug, Clone)]
pub struct FixedInterval {
    max_attempts: u32,
    interval: Duration,
}

impl FixedInterval {
    /// 创建固定间隔策略
    ///
    /// # 参数
    ///
    /// - `max_attempts`: 最大尝试次数
    /// - `interval`: 重试间隔
    pub fn new(max_attempts: u32, interval: Duration) -> Self {
        Self {
            max_attempts,
            interval,
        }
    }

    /// 执行带重试的操作
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

/// 不重试策略
///
/// 失败立即返回，不进行任何重试。适合关键操作或需要快速失败的场景。
#[derive(Debug, Clone)]
pub struct NoRetry;

impl NoRetry {
    /// 执行操作（不重试）
    pub async fn execute<F, Fut, T>(&self, operation: F) -> EmailResult<T>
    where
        F: Fn() -> Fut + Send,
        Fut: std::future::Future<Output = EmailResult<T>> + Send,
        T: Send,
    {
        operation().await
    }
}

/// 自适应重试策略
///
/// 结合熔断器的重试策略，在连续失败后自动熔断，防止级联故障。
///
/// # 工作原理
///
/// 1. 正常状态下，使用指数退避策略重试
/// 2. 连续失败达到阈值后，熔断器打开，快速失败
/// 3. 等待恢复时间后，熔断器进入半开状态，允许尝试
/// 4. 如果成功，熔断器关闭；如果失败，熔断器重新打开
///
/// # 示例
///
/// ```rust
/// use email_service::providers::AdaptiveRetry;
///
/// let strategy = AdaptiveRetry::new(5);
/// ```
#[derive(Debug, Clone)]
pub struct AdaptiveRetry {
    base_strategy: ExponentialBackoff,
    circuit_breaker: CircuitBreaker,
}

impl AdaptiveRetry {
    /// 创建自适应重试策略
    ///
    /// # 参数
    ///
    /// - `max_attempts`: 最大尝试次数
    ///
    /// 默认熔断器配置：
    /// - 失败阈值: 5 次
    /// - 恢复时间: 60 秒
    pub fn new(max_attempts: u32) -> Self {
        Self {
            base_strategy: ExponentialBackoff::new(max_attempts),
            circuit_breaker: CircuitBreaker::new(5, Duration::from_secs(60)),
        }
    }

    /// 执行带重试和熔断保护的操作
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

/// 熔断器
///
/// 实现熔断器模式，防止级联故障。
///
/// # 状态机
///
/// ```text
/// ┌─────────┐    失败达到阈值    ┌─────────┐
/// │  Closed │ ─────────────────> │  Open   │
/// │ (正常)  │                    │ (熔断)  │
/// └─────────┘                    └─────────┘
///      ^                              │
///      │ 成功                         │ 超时后
///      │                              ▼
///      │                        ┌──────────┐
///      └─────────────────────── │ HalfOpen │
///            成功                │ (半开)   │
///                               └──────────┘
///                                    │
///                                    │ 失败
///                                    ▼
///                               ┌─────────┐
///                               │  Open   │
///                               └─────────┘
/// ```
///
/// # 示例
///
/// ```rust
/// use email_service::providers::CircuitBreaker;
/// use std::time::Duration;
///
/// let breaker = CircuitBreaker::new(5, Duration::from_secs(60));
///
/// // 检查是否可以执行
/// breaker.check()?;
///
/// // 记录成功
/// breaker.record_success();
///
/// // 记录失败
/// breaker.record_failure();
/// # Ok::<(), email_service::error::EmailError>(())
/// ```
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    failure_threshold: u32,
    failure_count: std::sync::Arc<parking_lot::Mutex<u32>>,
    reset_timeout: Duration,
    last_failure_time: std::sync::Arc<parking_lot::Mutex<Option<std::time::Instant>>>,
    state: std::sync::Arc<parking_lot::Mutex<CircuitState>>,
}

/// 熔断器状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CircuitState {
    /// 关闭状态（正常）
    ///
    /// 允许所有请求通过。
    Closed,

    /// 打开状态（熔断）
    ///
    /// 拒绝所有请求，快速失败。
    Open,

    /// 半开状态
    ///
    /// 允许部分请求通过，用于探测服务是否恢复。
    HalfOpen,
}

impl CircuitBreaker {
    /// 创建熔断器
    ///
    /// # 参数
    ///
    /// - `failure_threshold`: 触发熔断的连续失败次数
    /// - `reset_timeout`: 熔断后等待恢复的时间
    ///
    /// # 示例
    ///
    /// ```rust
    /// use email_service::providers::CircuitBreaker;
    /// use std::time::Duration;
    ///
    /// // 连续失败 5 次后熔断，60 秒后尝试恢复
    /// let breaker = CircuitBreaker::new(5, Duration::from_secs(60));
    /// ```
    pub fn new(failure_threshold: u32, reset_timeout: Duration) -> Self {
        Self {
            failure_threshold,
            failure_count: std::sync::Arc::new(parking_lot::Mutex::new(0)),
            reset_timeout,
            last_failure_time: std::sync::Arc::new(parking_lot::Mutex::new(None)),
            state: std::sync::Arc::new(parking_lot::Mutex::new(CircuitState::Closed)),
        }
    }

    /// 检查是否可以执行操作
    ///
    /// # 返回值
    ///
    /// - `Ok(())`: 可以执行
    /// - `Err(EmailError::ProviderUnavailable)`: 熔断器打开，拒绝执行
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

    /// 记录成功
    ///
    /// 重置失败计数，关闭熔断器。
    pub fn record_success(&self) {
        let mut state = self.state.lock();
        let mut count = self.failure_count.lock();

        *count = 0;
        *state = CircuitState::Closed;
    }

    /// 记录失败
    ///
    /// 增加失败计数，达到阈值后打开熔断器。
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
