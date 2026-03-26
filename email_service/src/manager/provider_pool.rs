use dashmap::DashMap;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use crate::error::{EmailError, EmailResult};
use crate::providers::BoxedEmailProvider;

pub struct ProviderPool {
    providers: DashMap<String, BoxedEmailProvider>,
    health_status: DashMap<String, HealthStatus>,
    #[allow(dead_code)]
    config: crate::config::PoolConfig,
}

#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub is_healthy: bool,
    pub last_check: Instant,
    pub consecutive_failures: u32,
    pub last_error: Option<String>,
}

impl HealthStatus {
    pub fn new() -> Self {
        Self {
            is_healthy: true,
            last_check: Instant::now(),
            consecutive_failures: 0,
            last_error: None,
        }
    }
}

impl ProviderPool {
    pub fn new(config: crate::config::PoolConfig) -> Self {
        Self {
            providers: DashMap::new(),
            health_status: DashMap::new(),
            config,
        }
    }

    pub fn register(&self, name: String, provider: BoxedEmailProvider) {
        self.providers.insert(name.clone(), provider);
        self.health_status.insert(name, HealthStatus::new());
    }

    pub fn get(&self, name: &str) -> EmailResult<BoxedEmailProvider> {
        self.providers
            .get(name)
            .map(|p| p.clone())
            .ok_or_else(|| EmailError::ProviderNotFound(name.to_string()))
    }

    pub fn get_healthy(&self, name: &str) -> EmailResult<BoxedEmailProvider> {
        let provider = self.get(name)?;
        
        let is_healthy = self.health_status
            .get(name)
            .map(|s| s.is_healthy)
            .unwrap_or(true);

        if !is_healthy {
            return Err(EmailError::ProviderUnavailable(name.to_string()));
        }

        Ok(provider)
    }

    pub fn get_best_provider(&self) -> Option<BoxedEmailProvider> {
        self.providers
            .iter()
            .filter(|entry| {
                let name = entry.key();
                self.health_status
                    .get(name)
                    .map(|s| s.is_healthy)
                    .unwrap_or(true)
            })
            .max_by_key(|entry| entry.value().priority())
            .map(|entry| entry.value().clone())
    }

    pub fn get_providers_by_priority(&self) -> Vec<BoxedEmailProvider> {
        let mut providers: Vec<_> = self.providers
            .iter()
            .filter(|entry| {
                let name = entry.key();
                self.health_status
                    .get(name)
                    .map(|s| s.is_healthy)
                    .unwrap_or(true)
            })
            .map(|entry| entry.value().clone())
            .collect();

        providers.sort_by(|a, b| b.priority().cmp(&a.priority()));
        providers
    }

    pub fn mark_healthy(&self, name: &str) {
        if let Some(mut status) = self.health_status.get_mut(name) {
            status.is_healthy = true;
            status.consecutive_failures = 0;
            status.last_check = Instant::now();
            status.last_error = None;
        }
    }

    pub fn mark_unhealthy(&self, name: &str, error: &str) {
        if let Some(mut status) = self.health_status.get_mut(name) {
            status.is_healthy = false;
            status.consecutive_failures += 1;
            status.last_check = Instant::now();
            status.last_error = Some(error.to_string());
        }
    }

    pub fn get_health_status(&self, name: &str) -> Option<HealthStatus> {
        self.health_status.get(name).map(|s| s.clone())
    }

    pub fn list_providers(&self) -> Vec<String> {
        self.providers.iter().map(|entry| entry.key().clone()).collect()
    }

    pub fn remove(&self, name: &str) -> Option<BoxedEmailProvider> {
        self.health_status.remove(name);
        self.providers.remove(name).map(|(_, v)| v)
    }

    pub fn clear(&self) {
        self.providers.clear();
        self.health_status.clear();
    }

    pub fn len(&self) -> usize {
        self.providers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }

    pub async fn health_check_all(&self) -> HashMap<String, bool> {
        let mut results = HashMap::new();

        for entry in self.providers.iter() {
            let name = entry.key().clone();
            let provider = entry.value().clone();

            let result = match provider.health_check().await {
                Ok(healthy) => healthy,
                Err(e) => {
                    tracing::warn!("Health check failed for provider '{}': {}", name, e);
                    false
                }
            };

            if result {
                self.mark_healthy(&name);
            } else {
                self.mark_unhealthy(&name, "Health check failed");
            }

            results.insert(name, result);
        }

        results
    }
}

pub struct ProviderSelector {
    pool: Arc<ProviderPool>,
    strategy: SelectionStrategy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionStrategy {
    Priority,
    RoundRobin,
    Random,
    LeastLoaded,
}

impl ProviderSelector {
    pub fn new(pool: Arc<ProviderPool>, strategy: SelectionStrategy) -> Self {
        Self { pool, strategy }
    }

    pub fn select(&self) -> Option<BoxedEmailProvider> {
        match self.strategy {
            SelectionStrategy::Priority => self.select_by_priority(),
            SelectionStrategy::RoundRobin => self.select_round_robin(),
            SelectionStrategy::Random => self.select_random(),
            SelectionStrategy::LeastLoaded => self.select_least_loaded(),
        }
    }

    pub fn select_by_name(&self, name: &str) -> EmailResult<BoxedEmailProvider> {
        self.pool.get_healthy(name)
    }

    fn select_by_priority(&self) -> Option<BoxedEmailProvider> {
        self.pool.get_best_provider()
    }

    fn select_round_robin(&self) -> Option<BoxedEmailProvider> {
        static COUNTER: RwLock<usize> = RwLock::new(0);
        
        let providers = self.pool.get_providers_by_priority();
        if providers.is_empty() {
            return None;
        }

        let mut counter = COUNTER.write();
        let index = *counter % providers.len();
        *counter = (*counter + 1) % providers.len();
        
        Some(providers[index].clone())
    }

    fn select_random(&self) -> Option<BoxedEmailProvider> {
        use rand::seq::SliceRandom;
        
        let mut providers = self.pool.get_providers_by_priority();
        providers.shuffle(&mut rand::thread_rng());
        providers.into_iter().next()
    }

    fn select_least_loaded(&self) -> Option<BoxedEmailProvider> {
        self.pool.get_best_provider()
    }
}
