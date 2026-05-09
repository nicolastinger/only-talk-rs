use std::sync::Arc;

use crc::Crc;
use dashmap::DashMap;

use crate::models::quic_connection::QuicConnection;

pub mod chat_node;
pub mod config;
pub mod init_server;
pub mod lifecycle;
pub mod models;
pub mod msg_service;
pub mod nat_ip;
pub(crate) mod quic_client;
pub(crate) mod quic_server;
pub(crate) mod set_server;
pub mod state;
pub mod tls_monitor;

/// 连接映射类型别名，便于跨 crate 使用
pub type ConnectionsMap = Arc<DashMap<String, QuicConnection>>;

// Re-exports
pub use chat_node::ChatNode;
pub use config::ChatNodeConfig;
pub use lifecycle::ServiceLifecycle;
pub use state::{ServiceError, ServiceState};

// 创建CRC-16/X25计算器
pub const X25: Crc<u16> = Crc::<u16>::new(&crc::CRC_16_IBM_SDLC);
