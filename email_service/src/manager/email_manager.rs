use std::sync::Arc;
use std::collections::HashMap;

use crate::config::{EmailServiceConfig, ProviderConfig};
use crate::error::{EmailError, EmailResult};
use crate::models::{Email, SendResult};
use crate::providers::{
    BoxedEmailProvider, RetryStrategy, 
    AliyunEmailProvider, TencentEmailProvider, 
    AwsSesEmailProvider, SmtpEmailProvider,
};
use super::provider_pool::{ProviderPool, ProviderSelector, SelectionStrategy};

pub struct EmailManager {
    pool: Arc<ProviderPool>,
    selector: ProviderSelector,
    retry_strategy: RetryStrategy,
    config: EmailServiceConfig,
}

impl EmailManager {
    pub fn new(config: EmailServiceConfig) -> EmailResult<Self> {
        let pool = Arc::new(ProviderPool::new(config.pool.clone()));
        
        Self::register_providers(&pool, &config.providers)?;
        
        let selector = ProviderSelector::new(pool.clone(), SelectionStrategy::Priority);
        let retry_strategy = RetryStrategy::from_config(&config.retry);

        Ok(Self {
            pool,
            selector,
            retry_strategy,
            config,
        })
    }

    pub fn builder() -> EmailManagerBuilder {
        EmailManagerBuilder::default()
    }

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

    fn create_provider(config: &ProviderConfig) -> EmailResult<BoxedEmailProvider> {
        match config {
            ProviderConfig::Aliyun(cfg) => AliyunEmailProvider::boxed(cfg.clone()),
            ProviderConfig::Tencent(cfg) => TencentEmailProvider::boxed(cfg.clone()),
            ProviderConfig::AwsSes(cfg) => AwsSesEmailProvider::boxed(cfg.clone()),
            ProviderConfig::Smtp(cfg) => SmtpEmailProvider::boxed(cfg.clone()),
        }
    }

    pub async fn send(&self, email: &Email) -> EmailResult<SendResult> {
        let provider = self.get_provider(email)?;
        
        provider.validate_email(email)?;

        let provider_name = provider.name().to_string();
        let result = self.retry_strategy.execute(|| {
            let provider = provider.clone();
            let email = email.clone();
            async move { provider.send(&email).await }
        }).await;

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

    pub async fn send_batch(&self, emails: &[Email]) -> Vec<EmailResult<SendResult>> {
        let mut results = Vec::with_capacity(emails.len());
        
        for email in emails {
            results.push(self.send(email).await);
        }

        results
    }

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

        Err(last_error.unwrap_or_else(|| {
            EmailError::ProviderUnavailable("All providers failed".to_string())
        }))
    }

    fn get_provider(&self, email: &Email) -> EmailResult<BoxedEmailProvider> {
        if let Some(ref provider_name) = email.provider {
            self.selector.select_by_name(provider_name)
        } else if let Some(ref default) = self.config.default_provider {
            self.selector.select_by_name(default)
        } else {
            self.selector.select()
                .ok_or_else(|| EmailError::ProviderNotFound("No provider available".to_string()))
        }
    }

    pub async fn health_check(&self) -> HashMap<String, bool> {
        self.pool.health_check_all().await
    }

    pub fn get_provider_status(&self, name: &str) -> Option<bool> {
        self.pool.get_health_status(name).map(|s| s.is_healthy)
    }

    pub fn list_providers(&self) -> Vec<String> {
        self.pool.list_providers()
    }

    pub fn register_provider(&self, name: String, config: ProviderConfig) -> EmailResult<()> {
        let provider = Self::create_provider(&config)?;
        self.pool.register(name, provider);
        Ok(())
    }

    pub fn remove_provider(&self, name: &str) -> Option<BoxedEmailProvider> {
        self.pool.remove(name)
    }

    pub fn set_selection_strategy(&mut self, strategy: SelectionStrategy) {
        self.selector = ProviderSelector::new(self.pool.clone(), strategy);
    }

    pub fn with_retry_strategy(mut self, strategy: RetryStrategy) -> Self {
        self.retry_strategy = strategy;
        self
    }
}

#[derive(Default)]
pub struct EmailManagerBuilder {
    config: EmailServiceConfig,
}

impl EmailManagerBuilder {
    pub fn default_provider(mut self, name: impl Into<String>) -> Self {
        self.config.default_provider = Some(name.into());
        self
    }

    pub fn provider(mut self, name: impl Into<String>, config: ProviderConfig) -> Self {
        self.config.providers.insert(name.into(), config);
        self
    }

    pub fn retry_config(mut self, retry: crate::config::RetryConfig) -> Self {
        self.config.retry = retry;
        self
    }

    pub fn pool_config(mut self, pool: crate::config::PoolConfig) -> Self {
        self.config.pool = pool;
        self
    }

    pub fn rate_limit_config(mut self, rate_limit: crate::config::RateLimitConfig) -> Self {
        self.config.rate_limit = rate_limit;
        self
    }

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
