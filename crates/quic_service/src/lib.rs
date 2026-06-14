use std::sync::Arc;

use dashmap::DashMap;

use crate::models::quic_connection::QuicConnection;

pub mod external;
pub mod init_server;
pub mod internal;
pub mod models;
pub mod msg_service;
pub mod nat_ip;

/// Connection map type alias for cross-crate usage
pub type ConnectionsMap = Arc<DashMap<String, QuicConnection>>;

// Re-exports
pub use external::{ChatNode, ChatNodeConfig, ServiceError, ServiceLifecycle, ServiceState};
pub use internal::{InternalQuicConfig, run_internal_server};

// Re-export X25 from common (moved to shared crate)
pub use common::utils::text_msg::X25;
