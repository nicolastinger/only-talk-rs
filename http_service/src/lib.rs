use std::sync::Arc;
use deadpool_redis::Pool;

pub mod http_service;
pub mod utils;
pub mod common;

rust_i18n::i18n!("locales");

pub struct AppState {
    pub redis_pool: Arc<Pool>,
}