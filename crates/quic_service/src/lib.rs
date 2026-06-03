use std::sync::Arc;

use crc::Crc;
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

// Create CRC-16/X25 calculator
pub const X25: Crc<u16> = Crc::<u16>::new(&crc::CRC_16_IBM_SDLC);
