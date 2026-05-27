use std::sync::Arc;

use async_trait::async_trait;
use dashmap::DashMap;
use quinn::Endpoint;
use tokio::sync::{Mutex, RwLock, watch};
use tracing::info;

use super::config::ChatNodeConfig;
use super::lifecycle::ServiceLifecycle;
use crate::models::quic_connection::QuicConnection;
use super::quic_server::run_server;
use super::set_server::make_server_endpoint;
use super::state::{ServiceError, ServiceState};
use super::tls_monitor::start_tls_monitor;

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

    /// 获取连接映射的引用，用于传递给其他服务
    pub fn connections(&self) -> Arc<DashMap<String, QuicConnection>> {
        self.connections.clone()
    }

    /// 获取配置的只读引用
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
        info!("[{}] 正在初始化...", self.name());

        let (endpoint, _server_cert) = make_server_endpoint(
            self.config.bind_address,
            &self.config.cert_path,
            &self.config.key_path,
        )
        .map_err(|e| ServiceError::Config(format!("创建QUIC端点失败: {}", e)))?;

        *self.endpoint.write().await = Some(endpoint);

        self.state.write().await.transition_to(ServiceState::Running)?;
        info!(
            "[{}] 初始化完成，监听地址: {}",
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

        info!("[{}] 正在启动...", self.name());

        let endpoint = {
            let ep = self.endpoint.read().await;
            ep.clone()
                .ok_or_else(|| ServiceError::Config("端点未初始化".to_string()))?
        };
        let endpoint = Arc::new(endpoint);

        // 创建 shutdown channel
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        {
            let mut tx = self.shutdown_tx.lock().await;
            *tx = Some(shutdown_tx);
        }

        // 启动 TLS 证书监控
        start_tls_monitor(
            endpoint.clone(),
            shutdown_rx.clone(),
            self.config.cert_path.clone(),
            self.config.key_path.clone(),
            self.config.cert_watch_interval_secs,
            self.config.cert_expiry_warning_days,
            self.config.cert_expiry_check_interval_secs,
        );

        // 启动 accept loop
        let connections = self.connections.clone();
        let config = self.config.clone();
        tokio::spawn(async move {
            run_server(endpoint, connections, config, shutdown_rx).await;
        });

        info!("[{}] 服务已启动", self.name());
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
        info!("[{}] 正在优雅关闭...", self.name());

        // 发送 shutdown 信号
        {
            let mut tx = self.shutdown_tx.lock().await;
            if let Some(tx) = tx.take() {
                let _ = tx.send(true);
            }
        }

        // 关闭 endpoint，让 accept loop 退出
        {
            let mut ep = self.endpoint.write().await;
            if let Some(endpoint) = ep.take() {
                endpoint.close(0u32.into(), b"server shutdown");
            }
        }

        // 等待一小段时间让 accept loop 退出
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        self.state.write().await.transition_to(ServiceState::Stopped)?;
        info!("[{}] 已关闭", self.name());
        Ok(())
    }

    fn status(&self) -> ServiceState {
        self.state
            .try_read()
            .map(|s| *s)
            .unwrap_or(ServiceState::Running)
    }
}
