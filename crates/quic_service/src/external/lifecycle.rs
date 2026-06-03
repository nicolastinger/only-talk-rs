use async_trait::async_trait;

use super::state::{ServiceError, ServiceState};

/// Service lifecycle trait.
/// Defines the complete lifecycle methods from creation to destruction.
#[async_trait]
pub trait ServiceLifecycle: Send + Sync {
    /// Service name
    fn name(&self) -> &str;

    /// Initialize resources: parse config, create endpoints, start background monitoring, etc.
    /// State must be Uninitialized.
    async fn init(&mut self) -> Result<(), ServiceError>;

    /// Start service main loop (non-blocking, runs in background).
    /// State must be Initializing or Running.
    async fn start(&self) -> Result<(), ServiceError>;

    /// Graceful shutdown: send shutdown signal, wait for background tasks to complete.
    /// State must be Running.
    async fn stop(&self) -> Result<(), ServiceError>;

    /// Return current lifecycle state
    fn status(&self) -> ServiceState;
}
