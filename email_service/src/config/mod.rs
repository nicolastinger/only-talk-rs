//! 邮件服务配置模块
//!
//! 本模块定义了邮件服务的所有配置项，包括：
//!
//! - [`EmailServiceConfig`]: 服务总配置
//! - [`RetryConfig`]: 重试策略配置
//! - [`PoolConfig`]: 连接池配置
//! - [`RateLimitConfig`]: 速率限制配置
//! - [`ProviderConfig`]: 服务商配置（见 [`provider_config`] 模块）
//!
//! # 配置示例
//!
//! ```rust
//! use email_service::config::{EmailServiceConfig, RetryConfig, PoolConfig};
//!
//! let config = EmailServiceConfig {
//!     default_provider: Some("aliyun".to_string()),
//!     retry: RetryConfig {
//!         max_attempts: 5,
//!         initial_delay_ms: 200,
//!         ..Default::default()
//!     },
//!     ..Default::default()
//! };
//! ```

mod provider_config;

pub use provider_config::*;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 邮件服务总配置
///
/// 包含邮件服务的所有配置项，可通过 JSON/YAML 文件加载或手动构建。
///
/// # 字段说明
///
/// | 字段 | 说明 | 默认值 |
/// |------|------|--------|
/// | `default_provider` | 默认服务商名称 | `None` |
/// | `providers` | 服务商配置映射 | 空 HashMap |
/// | `retry` | 重试配置 | 见 [`RetryConfig`] |
/// | `pool` | 连接池配置 | 见 [`PoolConfig`] |
/// | `rate_limit` | 速率限制配置 | 见 [`RateLimitConfig`] |
///
/// # 示例
///
/// ```rust
/// use email_service::config::EmailServiceConfig;
///
/// // 使用默认配置
/// let config = EmailServiceConfig::default();
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailServiceConfig {
    /// 默认服务商名称
    ///
    /// 当调用 [`EmailManager::send`] 时，会使用此服务商。
    /// 如果未设置，则使用优先级最高的服务商。
    pub default_provider: Option<String>,

    /// 服务商配置映射
    ///
    /// 键为服务商名称，值为服务商配置。
    /// 支持多个服务商配置，便于故障转移。
    pub providers: HashMap<String, ProviderConfig>,

    /// 重试策略配置
    pub retry: RetryConfig,

    /// 连接池配置
    pub pool: PoolConfig,

    /// 速率限制配置
    pub rate_limit: RateLimitConfig,
}

impl Default for EmailServiceConfig {
    fn default() -> Self {
        Self {
            default_provider: None,
            providers: HashMap::new(),
            retry: RetryConfig::default(),
            pool: PoolConfig::default(),
            rate_limit: RateLimitConfig::default(),
        }
    }
}

/// 重试策略配置
///
/// 控制邮件发送失败时的重试行为。
///
/// # 重试间隔计算
///
/// 使用指数退避算法计算重试间隔：
///
/// ```text
/// delay = min(initial_delay * multiplier^(attempt-1), max_delay)
/// ```
///
/// 如果启用 `jitter`，会在计算结果上添加随机抖动，防止惊群效应。
///
/// # 示例
///
/// ```rust
/// use email_service::config::RetryConfig;
///
/// let config = RetryConfig {
///     max_attempts: 5,        // 最多重试 5 次
///     initial_delay_ms: 100,  // 初始延迟 100ms
///     max_delay_ms: 60000,    // 最大延迟 60s
///     multiplier: 2.0,        // 每次延迟翻倍
///     jitter: true,           // 启用随机抖动
/// };
/// ```
///
/// # 重试时间线示例
///
/// | 尝试次数 | 基础延迟 | 含抖动延迟（约） |
/// |----------|----------|------------------|
/// | 1 | 100ms | 80-120ms |
/// | 2 | 200ms | 160-240ms |
/// | 3 | 400ms | 320-480ms |
/// | 4 | 800ms | 640-960ms |
/// | 5 | 1600ms | 1280-1920ms |
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// 最大重试次数
    ///
    /// 包括首次发送，总尝试次数为 `max_attempts + 1`。
    /// 设为 0 表示不重试。
    pub max_attempts: u32,

    /// 初始延迟（毫秒）
    ///
    /// 第一次重试前的等待时间。
    pub initial_delay_ms: u64,

    /// 最大延迟（毫秒）
    ///
    /// 重试延迟的上限，防止等待时间过长。
    pub max_delay_ms: u64,

    /// 延迟乘数
    ///
    /// 每次重试后延迟乘以此值。通常设为 2.0（翻倍）。
    pub multiplier: f64,

    /// 是否启用随机抖动
    ///
    /// 启用后会在延迟基础上添加 ±20% 的随机抖动，
    /// 防止多个客户端同时重试（惊群效应）。
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 100,
            max_delay_ms: 30000,
            multiplier: 2.0,
            jitter: true,
        }
    }
}

/// 连接池配置
///
/// 控制 HTTP 连接池的行为，优化连接复用。
///
/// # 为什么需要连接池？
///
/// - **减少连接开销**: 复用 TCP 连接，避免频繁握手
/// - **控制资源使用**: 限制最大连接数，防止资源耗尽
/// - **提高性能**: 保持空闲连接，减少延迟
///
/// # 示例
///
/// ```rust
/// use email_service::config::PoolConfig;
///
/// let config = PoolConfig {
///     max_connections: 20,      // 最大 20 个连接
///     min_connections: 2,       // 最少保持 2 个空闲连接
///     connection_timeout_ms: 3000,  // 连接超时 3s
///     idle_timeout_ms: 60000,   // 空闲连接 60s 后关闭
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    /// 最大连接数
    ///
    /// 连接池中允许的最大连接数量。
    /// 超过此限制的请求会等待连接释放。
    pub max_connections: usize,

    /// 最小空闲连接数
    ///
    /// 连接池中保持的最小空闲连接数量。
    /// 便于快速响应新请求。
    pub min_connections: usize,

    /// 连接超时（毫秒）
    ///
    /// 建立新连接的最大等待时间。
    /// 超时后返回错误。
    pub connection_timeout_ms: u64,

    /// 空闲超时（毫秒）
    ///
    /// 空闲连接在池中保持的最长时间。
    /// 超时后连接会被关闭。
    pub idle_timeout_ms: u64,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 10,
            min_connections: 1,
            connection_timeout_ms: 5000,
            idle_timeout_ms: 300000,
        }
    }
}

/// 速率限制配置
///
/// 控制邮件发送频率，防止触发服务商限制。
///
/// # 令牌桶算法
///
/// 使用令牌桶算法实现速率限制：
///
/// - 以固定速率向桶中添加令牌
/// - 每次发送邮件消耗一个令牌
/// - 桶满时令牌溢出
/// - 桶空时请求被阻塞或拒绝
///
/// # 示例
///
/// ```rust
/// use email_service::config::RateLimitConfig;
///
/// let config = RateLimitConfig {
///     enabled: true,
///     requests_per_second: 10,  // 每秒 10 封邮件
///     burst_size: 20,           // 允许突发 20 封
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// 是否启用速率限制
    pub enabled: bool,

    /// 每秒请求数
    ///
    /// 令牌桶的填充速率。
    /// 设为 10 表示每秒最多发送 10 封邮件。
    pub requests_per_second: u32,

    /// 突发容量
    ///
    /// 令牌桶的最大容量。
    /// 允许短时间内发送超过 `requests_per_second` 的邮件。
    pub burst_size: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            requests_per_second: 10,
            burst_size: 20,
        }
    }
}
