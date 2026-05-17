use std::sync::Arc;
use std::sync::OnceLock;

use deadpool_redis::Pool;
use lazy_static::lazy_static;
use rbatis::RBatis;
use tokio::sync::RwLock;

pub mod config_str;
pub mod config_manager;
pub mod marcos;
pub mod tracing;
pub mod utils;

// Re-export
pub use utils::internal_quic_client::send_internal_quic_msg;
pub use utils::redis_utils::init_redis;
pub use utils::server_count_sync::{get_server_count, start_server_count_sync, SERVER_COUNT};
pub use utils::sql_utils::init_sql_pool;

// Re-export entity 的 models，使外部只需依赖 core 即可访问 DB 实体
pub use entity::models;

/// 确保 Redis / SQL 只初始化一次
static REDIS_INIT_ONCE: OnceLock<()> = OnceLock::new();
static SQL_INIT_ONCE: OnceLock<()> = OnceLock::new();

lazy_static! {
    pub static ref REDIS_CLIENT: Arc<RwLock<Option<Pool>>> = Arc::new(RwLock::new(None));
    pub static ref RBATIS_DATABASE: Arc<RwLock<Option<RBatis>>> = Arc::new(RwLock::new(None));
}
