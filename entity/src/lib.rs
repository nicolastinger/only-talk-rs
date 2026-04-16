use std::sync::Arc;

use deadpool_redis::Pool;
use lazy_static::lazy_static;
use rbatis::RBatis;
use tokio::sync::RwLock;

pub mod config_str;
pub mod config_manager;
pub mod marcos;
pub mod models;
pub mod utils;

// 创建一个quic服务器维护列表全局变量，使用 RwLock 包装，后期采用dashMap
// 使用 lazy_static 初始化全局共享变量
lazy_static! {
    pub static ref REDIS_CLIENT: Arc<RwLock<Option<Pool>>> = Arc::new(RwLock::new(None));
    pub static ref RBATIS_DATABASE: Arc<RwLock<Option<RBatis>>> = Arc::new(RwLock::new(None));
}
