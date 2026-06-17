use std::sync::Arc;

use async_trait::async_trait;
use dashmap::DashMap;
use quinn::Endpoint;
use tokio::sync::{Mutex, RwLock, watch};
use tracing::info;

use super::config::ChatNodeConfig;
use super::lifecycle::ServiceLifecycle;
use super::quic_server::run_server;
use super::set_server::make_server_endpoint;
use super::state::{ServiceError, ServiceState};
use super::tls_monitor::start_tls_monitor;
use crate::models::quic_connection::QuicConnection;

pub struct ChatNode {
    config: ChatNodeConfig,
    state: RwLock<ServiceState>,
    endpoint: RwLock<Option<Endpoint>>,
    connections: Arc<DashMap<String, QuicConnection>>,
    shutdown_tx: Mutex<Option<watch::Sender<bool>>>,
    name: String,
}

impl ChatNode {
    pub fn new(config: ChatNodeConfig) -> Self {
        Self {
            name: config.server_name.clone(),
            config,
            state: RwLock::new(ServiceState::Uninitialized),
            endpoint: RwLock::new(None),
            connections: Arc::new(DashMap::new()),
            shutdown_tx: Mutex::new(None),
        }
    }

    /// Get a reference to the connection map, for passing to other services
    pub fn connections(&self) -> Arc<DashMap<String, QuicConnection>> {
        self.connections.clone()
    }

    /// Get a read-only reference to the config
    pub fn config(&self) -> &ChatNodeConfig {
        &self.config
    }
}

#[async_trait]
impl ServiceLifecycle for ChatNode {
    fn name(&self) -> &str {
        &self.name
    }

    async fn init(&mut self) -> Result<(), ServiceError> {
        {
            let state = *self.state.read().await;
            if state != ServiceState::Uninitialized {
                return Err(ServiceError::InvalidStateTransition {
                    from: state,
                    to: ServiceState::Initializing,
                });
            }
        }
        self.state.write().await.transition_to(ServiceState::Initializing)?;
        info!("[{}] initializing...", self.name());

        let (endpoint, _server_cert) = make_server_endpoint(
            self.config.bind_address,
            &self.config.cert_path,
            &self.config.key_path,
        )
        .map_err(|e| ServiceError::Config(format!("Failed to create QUIC endpoint: {}", e)))?;

        *self.endpoint.write().await = Some(endpoint);

        self.state.write().await.transition_to(ServiceState::Running)?;
        info!(
            "[{}] Initialization complete, listening on: {}",
            self.name(),
            self.config.bind_address
        );
        Ok(())
    }

    async fn start(&self) -> Result<(), ServiceError> {
        {
            let state = *self.state.read().await;
            if state != ServiceState::Running && state != ServiceState::Initializing {
                return Err(ServiceError::InvalidStateTransition {
                    from: state,
                    to: ServiceState::Running,
                });
            }
        }

        info!("[{}] starting...", self.name());

        let endpoint = {
            let ep = self.endpoint.read().await;
            ep.clone()
                .ok_or_else(|| ServiceError::Config("Endpoint not initialized".to_string()))?
        };
        let endpoint = Arc::new(endpoint);

        // Create shutdown channel
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        {
            let mut tx = self.shutdown_tx.lock().await;
            *tx = Some(shutdown_tx);
        }

        // Start TLS cert monitoring
        start_tls_monitor(
            endpoint.clone(),
            shutdown_rx.clone(),
            self.config.cert_path.clone(),
            self.config.key_path.clone(),
            self.config.cert_watch_interval_secs,
            self.config.cert_expiry_warning_days,
            self.config.cert_expiry_check_interval_secs,
        );

        // Start accept loop
        let connections = self.connections.clone();
        let config = self.config.clone();
        tokio::spawn(async move {
            run_server(endpoint, connections, config, shutdown_rx).await;
        });

        info!("[{}] service started", self.name());
        Ok(())
    }

    async fn stop(&self) -> Result<(), ServiceError> {
        {
            let state = *self.state.read().await;
            if state != ServiceState::Running {
                return Err(ServiceError::InvalidStateTransition {
                    from: state,
                    to: ServiceState::Stopping,
                });
            }
        }
        self.state.write().await.transition_to(ServiceState::Stopping)?;
        info!("[{}] shutting down gracefully...", self.name());

        // Send shutdown signal
        {
            let mut tx = self.shutdown_tx.lock().await;
            if let Some(tx) = tx.take() {
                let _ = tx.send(true);
            }
        }

        // Close endpoint, let accept loop exit
        {
            let mut ep = self.endpoint.write().await;
            if let Some(endpoint) = ep.take() {
                endpoint.close(0u32.into(), b"server shutdown");
            }
        }

        // Wait briefly for accept loop to exit
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        self.state.write().await.transition_to(ServiceState::Stopped)?;
        info!("[{}] closed", self.name());
        Ok(())
    }

    fn status(&self) -> ServiceState {
        self.state.try_read().map(|s| *s).unwrap_or(ServiceState::Running)
    }
}
