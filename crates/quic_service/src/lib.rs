use std::sync::Arc;

use dashmap::DashMap;

use crate::models::quic_connection::QuicConnection;

pub mod external;
pub mod init_server;
pub mod internal;
pub mod models;
pub mod msg_service;
pub mod nat_ip;

/// 连接映射类型别名，供跨 crate 使用
pub type ConnectionsMap = Arc<DashMap<String, QuicConnection>>;

// 重新导出
// 从 common 重新导出 X25（已迁移到共享 crate）
pub use common::utils::text_msg::X25;
pub use external::{ChatNode, ChatNodeConfig, ServiceError, ServiceLifecycle, ServiceState};
pub use internal::{InternalQuicConfig, run_internal_server};
