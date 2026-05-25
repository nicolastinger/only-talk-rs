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

/// 连接映射类型别名，便于跨 crate 使用
pub type ConnectionsMap = Arc<DashMap<String, QuicConnection>>;

// Re-exports
pub use external::{ChatNode, ChatNodeConfig, ServiceError, ServiceLifecycle, ServiceState};
pub use internal::{InternalQuicConfig, run_internal_server};

// 创建CRC-16/X25计算器
pub const X25: Crc<u16> = Crc::<u16>::new(&crc::CRC_16_IBM_SDLC);
