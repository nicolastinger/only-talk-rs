//! 邮件管理器实现
//!
//! 本模块实现 [`EmailManager`] 和 [`EmailManagerBuilder`]。

use std::collections::HashMap;
use std::sync::Arc;

use super::provider_pool::{ProviderPool, ProviderSelector, SelectionStrategy};
use crate::config::{EmailServiceConfig, ProviderConfig};
use crate::error::{EmailError, EmailResult};
use crate::models::{Email, SendResult};
use crate::providers::{
    AliyunEmailProvider, AwsSesEmailProvider, BoxedEmailProvider, RetryStrategy, SmtpEmailProvider,
    TencentEmailProvider,
};

/// 邮件管理器
///
/// 邮件服务的核心组件，负责：
///
/// - 管理多个邮件服务商
/// - 选择合适的服务商发送邮件
/// - 处理重试逻辑
/// - 监控服务商健康状态
///
/// # 创建管理器
///
/// 推荐使用 Builder 模式创建：
///
/// ```rust
/// use email_service::manager::EmailManager;
/// use email_service::{ProviderConfig, AliyunConfig, RetryConfig};
///
/// let manager = EmailManager::builder()
///     .default_provider("aliyun")
///     .provider("aliyun", ProviderConfig::Aliyun(AliyunConfig {
///         access_key_id: "your_key".to_string(),
///         access_key_secret: "your_secret".to_string(),
///         region_id: "cn-hangzhou".to_string(),
///         account_name: "noreply@example.com".to_string(),
///         ..Default::default()
///     }))
///     .retry_config(RetryConfig {
///         max_attempts: 3,
///         ..Default::default()
///     })
///     .build()?;
/// # Ok::<(), email_service::error::EmailError>(())
/// ```
///
/// # 发送邮件
///
/// ```rust,no_run
/// use email_service::prelude::*;
///
/// async fn send_email(manager: &EmailManager) -> EmailResult<()> {
///     let email = Email::builder()
///         .from(EmailAddress::new("noreply@example.com")?)
///         .to(EmailAddress::new("user@example.com")?)
///         .subject("测试邮件")
///         .text_body("这是邮件正文")
///         .build()?;
///
///     // 普通发送
///     let result = manager.send(&email).await?;
///
///     // 带故障转移发送
///     let result = manager.send_with_fallback(&email).await?;
///
///     Ok(())
/// }
/// # Ok::<(), email_service::error::EmailError>(())
/// ```
///
/// # 服务商选择
///
/// 邮件管理器按以下优先级选择服务商：
///
/// 1. 邮件中指定的服务商（`email.provider`）
/// 2. 配置的默认服务商（`default_provider`）
/// 3. 按优先级排序的第一个可用服务商
pub struct EmailManager {
    /// 服务商连接池
    pool: Arc<ProviderPool>,

    /// 服务商选择器
    selector: ProviderSelector,

    /// 重试策略
    retry_strategy: RetryStrategy,

    /// 服务配置
    config: EmailServiceConfig,
}

impl EmailManager {
    /// 创建邮件管理器
    ///
    /// # 参数
    ///
    /// - `config`: 服务配置
    ///
    /// # 错误
    ///
    /// 如果配置无效或服务商注册失败，返回错误。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use email_service::manager::EmailManager;
    /// use email_service::config::EmailServiceConfig;
    ///
    /// let config = EmailServiceConfig::default();
    /// let manager = EmailManager::new(config)?;
    /// # Ok::<(), email_service::error::EmailError>(())
    /// ```
    pub fn new(config: EmailServiceConfig) -> EmailResult<Self> {
        let pool = Arc::new(ProviderPool::new(config.pool.clone()));

        Self::register_providers(&pool, &config.providers)?;

        let selector = ProviderSelector::new(pool.clone(), SelectionStrategy::Priority);
        let retry_strategy = RetryStrategy::from_config(&config.retry);

        Ok(Self { pool, selector, retry_strategy, config })
    }

    /// 创建管理器构建器
    ///
    /// 返回一个新的 [`EmailManagerBuilder`] 实例。
    pub fn builder() -> EmailManagerBuilder {
        EmailManagerBuilder::default()
    }

    /// 注册服务商
    fn register_providers(
        pool: &Arc<ProviderPool>,
        providers: &HashMap<String, ProviderConfig>,
    ) -> EmailResult<()> {
        for (name, config) in providers {
            if !config.is_enabled() {
                tracing::info!("Provider '{}' is disabled, skipping", name);
                continue;
            }

            let provider = Self::create_provider(config)?;
            pool.register(name.clone(), provider);
            tracing::info!("Registered email provider: {}", name);
        }

        Ok(())
    }

    /// 根据配置创建服务商实例
    fn create_provider(config: &ProviderConfig) -> EmailResult<BoxedEmailProvider> {
        match config {
            ProviderConfig::Aliyun(cfg) => AliyunEmailProvider::boxed(cfg.clone()),
            ProviderConfig::Tencent(cfg) => TencentEmailProvider::boxed(cfg.clone()),
            ProviderConfig::AwsSes(cfg) => AwsSesEmailProvider::boxed(cfg.clone()),
            ProviderConfig::Smtp(cfg) => SmtpEmailProvider::boxed(cfg.clone()),
        }
    }

    /// 发送邮件
    ///
    /// 使用配置的服务商发送邮件，支持自动重试。
    ///
    /// # 参数
    ///
    /// - `email`: 要发送的邮件
    ///
    /// # 返回值
    ///
    /// - `Ok(SendResult)`: 发送结果（可能成功或失败）
    /// - `Err(EmailError)`: 发送异常
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use email_service::prelude::*;
    ///
    /// async fn send(manager: &EmailManager) -> EmailResult<()> {
    ///     let email = Email::builder()
    ///         .from(EmailAddress::new("noreply@example.com")?)
    ///         .to(EmailAddress::new("user@example.com")?)
    ///         .subject("测试")
    ///         .text_body("内容")
    ///         .build()?;
    ///
    ///     let result = manager.send(&email).await?;
    ///
    ///     if result.is_success() {
    ///         println!("发送成功: {:?}", result.message_id);
    ///     } else {
    ///         println!("发送失败: {:?}", result.error);
    ///     }
    ///
    ///     Ok(())
    /// }
    /// # Ok::<(), email_service::error::EmailError>(())
    /// ```
    pub async fn send(&self, email: &Email) -> EmailResult<SendResult> {
        let provider = self.get_provider(email)?;

        provider.validate_email(email)?;

        let provider_name = provider.name().to_string();
        let result = self
            .retry_strategy
            .execute(|| {
                let provider = provider.clone();
                let email = email.clone();
                async move { provider.send(&email).await }
            })
            .await;

        match &result {
            Ok(send_result) => {
                self.pool.mark_healthy(&provider_name);
                tracing::info!(
                    email_id = %email.id,
                    provider = %provider_name,
                    status = ?send_result.status,
                    "Email sent successfully"
                );
            }
            Err(e) => {
                self.pool.mark_unhealthy(&provider_name, &e.to_string());
                tracing::error!(
                    email_id = %email.id,
                    provider = %provider_name,
                    error = %e,
                    "Failed to send email"
                );
            }
        }

        result
    }

    /// 批量发送邮件
    ///
    /// 逐封发送邮件，返回每封邮件的发送结果。
    ///
    /// # 参数
    ///
    /// - `emails`: 要发送的邮件列表
    ///
    /// # 返回值
    ///
    /// 返回每封邮件的发送结果，顺序与输入一致。
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use email_service::prelude::*;
    ///
    /// async fn batch_send(manager: &EmailManager) {
    ///     let emails = vec![
    ///         // ... 邮件列表
    ///     ];
    ///
    ///     let results = manager.send_batch(&emails).await;
    ///
    ///     for (email, result) in emails.iter().zip(results.iter()) {
    ///         println!("邮件 {}: {:?}", email.id, result);
    ///     }
    /// }
    /// ```
    pub async fn send_batch(&self, emails: &[Email]) -> Vec<EmailResult<SendResult>> {
        let mut results = Vec::with_capacity(emails.len());

        for email in emails {
            results.push(self.send(email).await);
        }

        results
    }

    /// 带故障转移发送邮件
    ///
    /// 如果首选服务商失败，自动尝试下一个服务商。
    ///
    /// # 工作流程
    ///
    /// 1. 按优先级排序服务商
    /// 2. 尝试第一个服务商
    /// 3. 如果失败，尝试下一个服务商
    /// 4. 直到成功或所有服务商都失败
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use email_service::prelude::*;
    ///
    /// async fn send_with_fallback(manager: &EmailManager) -> EmailResult<()> {
    ///     let email = Email::builder()
    ///         .from(EmailAddress::new("noreply@example.com")?)
    ///         .to(EmailAddress::new("user@example.com")?)
    ///         .subject("重要通知")
    ///         .text_body("内容")
    ///         .build()?;
    ///
    ///     // 会尝试所有服务商直到成功
    ///     let result = manager.send_with_fallback(&email).await?;
    ///
    ///     Ok(())
    /// }
    /// # Ok::<(), email_service::error::EmailError>(())
    /// ```
    pub async fn send_with_fallback(&self, email: &Email) -> EmailResult<SendResult> {
        let providers = self.pool.get_providers_by_priority();

        if providers.is_empty() {
            return Err(EmailError::ProviderNotFound("No providers available".to_string()));
        }

        let mut last_error: Option<EmailError> = None;

        for provider in providers {
            let provider_name = provider.name().to_string();

            match provider.validate_email(email) {
                Ok(_) => {}
                Err(e) => {
                    last_error = Some(e);
                    continue;
                }
            }

            match provider.send(email).await {
                Ok(result) => {
                    self.pool.mark_healthy(&provider_name);
                    tracing::info!(
                        email_id = %email.id,
                        provider = %provider_name,
                        "Email sent successfully with fallback"
                    );
                    return Ok(result);
                }
                Err(e) => {
                    self.pool.mark_unhealthy(&provider_name, &e.to_string());
                    last_error = Some(e);
                    tracing::warn!(
                        email_id = %email.id,
                        provider = %provider_name,
                        error = %last_error.as_ref().unwrap(),
                        "Provider failed, trying next"
                    );
                }
            }
        }

        Err(last_error
            .unwrap_or_else(|| EmailError::ProviderUnavailable("All providers failed".to_string())))
    }

    /// 获取服务商
    fn get_provider(&self, email: &Email) -> EmailResult<BoxedEmailProvider> {
        if let Some(ref provider_name) = email.provider {
            self.selector.select_by_name(provider_name)
        } else if let Some(ref default) = self.config.default_provider {
            self.selector.select_by_name(default)
        } else {
            self.selector
                .select()
                .ok_or_else(|| EmailError::ProviderNotFound("No provider available".to_string()))
        }
    }

    /// 健康检查
    ///
    /// 检查所有服务商的健康状态。
    ///
    /// # 返回值
    ///
    /// 返回服务商名称到健康状态的映射。
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use email_service::manager::EmailManager;
    ///
    /// async fn check_health(manager: &EmailManager) {
    ///     let health = manager.health_check().await;
    ///
    ///     for (provider, is_healthy) in &health {
    ///         println!("{}: {}", provider, if *is_healthy { "健康" } else { "不健康" });
    ///     }
    /// }
    /// ```
    pub async fn health_check(&self) -> HashMap<String, bool> {
        self.pool.health_check_all().await
    }

    /// 获取服务商状态
    ///
    /// # 参数
    ///
    /// - `name`: 服务商名称
    ///
    /// # 返回值
    ///
    /// - `Some(true)`: 健康
    /// - `Some(false)`: 不健康
    /// - `None`: 服务商不存在
    pub fn get_provider_status(&self, name: &str) -> Option<bool> {
        self.pool.get_health_status(name).map(|s| s.is_healthy)
    }

    /// 列出所有服务商
    ///
    /// 返回已注册的服务商名称列表。
    pub fn list_providers(&self) -> Vec<String> {
        self.pool.list_providers()
    }

    /// 动态注册服务商
    ///
    /// 在运行时添加新的服务商。
    ///
    /// # 参数
    ///
    /// - `name`: 服务商名称
    /// - `config`: 服务商配置
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use email_service::manager::EmailManager;
    /// use email_service::{ProviderConfig, AliyunConfig, EmailResult};
    ///
    /// fn add_provider(manager: &EmailManager) -> EmailResult<()> {
    ///     manager.register_provider(
    ///         "new-aliyun".to_string(),
    ///         ProviderConfig::Aliyun(AliyunConfig {
    ///             access_key_id: "key".to_string(),
    ///             access_key_secret: "secret".to_string(),
    ///             region_id: "cn-hangzhou".to_string(),
    ///             account_name: "noreply@example.com".to_string(),
    ///             ..Default::default()
    ///         })
    ///     )
    /// }
    /// # Ok::<(), email_service::error::EmailError>(())
    /// ```
    pub fn register_provider(&self, name: String, config: ProviderConfig) -> EmailResult<()> {
        let provider = Self::create_provider(&config)?;
        self.pool.register(name, provider);
        Ok(())
    }

    /// 移除服务商
    ///
    /// # 参数
    ///
    /// - `name`: 服务商名称
    ///
    /// # 返回值
    ///
    /// 如果服务商存在，返回 `Some(provider)`；否则返回 `None`。
    pub fn remove_provider(&self, name: &str) -> Option<BoxedEmailProvider> {
        self.pool.remove(name)
    }

    /// 设置服务商选择策略
    ///
    /// # 参数
    ///
    /// - `strategy`: 选择策略
    pub fn set_selection_strategy(&mut self, strategy: SelectionStrategy) {
        self.selector = ProviderSelector::new(self.pool.clone(), strategy);
    }

    /// 设置重试策略
    ///
    /// # 参数
    ///
    /// - `strategy`: 重试策略
    pub fn with_retry_strategy(mut self, strategy: RetryStrategy) -> Self {
        self.retry_strategy = strategy;
        self
    }
}

/// 邮件管理器构建器
///
/// 使用 Builder 模式逐步构建 [`EmailManager`]。
///
/// # 示例
///
/// ```rust
/// use email_service::manager::EmailManager;
/// use email_service::{ProviderConfig, AliyunConfig, TencentConfig, RetryConfig};
///
/// let manager = EmailManager::builder()
///     .default_provider("aliyun")
///     .provider("aliyun", ProviderConfig::Aliyun(AliyunConfig {
///         access_key_id: "aliyun_key".to_string(),
///         access_key_secret: "aliyun_secret".to_string(),
///         region_id: "cn-hangzhou".to_string(),
///         account_name: "noreply@example.com".to_string(),
///         priority: 100,
///         ..Default::default()
///     }))
///     .provider("tencent", ProviderConfig::Tencent(TencentConfig {
///         secret_id: "tencent_id".to_string(),
///         secret_key: "tencent_key".to_string(),
///         region: "ap-guangzhou".to_string(),
///         sender: "noreply@example.com".to_string(),
///         priority: 90,
///         ..Default::default()
///     }))
///     .retry_config(RetryConfig {
///         max_attempts: 5,
///         ..Default::default()
///     })
///     .build()?;
/// # Ok::<(), email_service::error::EmailError>(())
/// ```
#[derive(Default)]
pub struct EmailManagerBuilder {
    config: EmailServiceConfig,
}

impl EmailManagerBuilder {
    /// 设置默认服务商
    ///
    /// 当邮件未指定服务商时使用。
    pub fn default_provider(mut self, name: impl Into<String>) -> Self {
        self.config.default_provider = Some(name.into());
        self
    }

    /// 添加服务商配置
    ///
    /// # 参数
    ///
    /// - `name`: 服务商名称（用于标识和日志）
    /// - `config`: 服务商配置
    pub fn provider(mut self, name: impl Into<String>, config: ProviderConfig) -> Self {
        self.config.providers.insert(name.into(), config);
        self
    }

    /// 设置重试配置
    pub fn retry_config(mut self, retry: crate::config::RetryConfig) -> Self {
        self.config.retry = retry;
        self
    }

    /// 设置连接池配置
    pub fn pool_config(mut self, pool: crate::config::PoolConfig) -> Self {
        self.config.pool = pool;
        self
    }

    /// 设置速率限制配置
    pub fn rate_limit_config(mut self, rate_limit: crate::config::RateLimitConfig) -> Self {
        self.config.rate_limit = rate_limit;
        self
    }

    /// 构建邮件管理器
    ///
    /// # 错误
    ///
    /// 如果配置无效，返回错误。
    pub fn build(self) -> EmailResult<EmailManager> {
        EmailManager::new(self.config)
    }
}

impl Clone for EmailManager {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            selector: ProviderSelector::new(self.pool.clone(), SelectionStrategy::Priority),
            retry_strategy: self.retry_strategy.clone(),
            config: self.config.clone(),
        }
    }
}
