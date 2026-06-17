use ::tracing::log;
use deadpool_redis::Pool;
use lazy_static::lazy_static;
use rbatis::RBatis;
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use std::sync::OnceLock;
use tokio::sync::RwLock;
use toml::Value;

pub mod config_manager;
pub mod config_str;
pub mod marcos;
pub mod tracing;
pub mod utils;

// Re-export
pub use utils::fatal_error::{fatal_panic, fatal_panic_async};
pub use utils::internal_quic_client::send_internal_quic_msg;
pub use utils::redis_utils::{init_redis, verify_redis};
pub use utils::server_count_sync::{SERVER_COUNT, get_server_count, start_server_count_sync};
pub use utils::sql_utils::init_sql_pool;

// Re-export entity models so external crates only need to depend on core to access DB entities
pub use entity::models;

/// Ensure Redis / SQL is initialized only once
static REDIS_INIT_ONCE: OnceLock<()> = OnceLock::new();
static SQL_INIT_ONCE: OnceLock<()> = OnceLock::new();

lazy_static! {
    pub static ref REDIS_CLIENT: Arc<RwLock<Option<Pool>>> = Arc::new(RwLock::new(None));
    pub static ref RBATIS_DATABASE: Arc<RwLock<Option<RBatis>>> = Arc::new(RwLock::new(None));
}

/// Replace environment variable placeholders ${VAR_NAME} in strings
/// Iterates up to 100 times to prevent malicious config from causing infinite loops
pub fn substitute_env_vars(content: String) -> String {
    let mut result = content;
    let mut iterations = 0;
    loop {
        if iterations > 100 {
            log::warn!(
                "env var substitution exceeded 100 iterations, possible circular reference, terminating early"
            );
            break;
        }
        iterations += 1;
        let Some(start) = result.find("${") else { break };
        let Some(end) = result[start..].find('}') else { break };
        let var_name = &result[start + 2..start + end];
        let var_value = std::env::var(var_name).unwrap_or_default();
        result = result.replace(&format!("${{{}}}", var_name), &var_value);
    }
    result
}

/// Read app config: replace env vars, parse TOML, populate global DashMap
/// Returns the substituted TOML string for callers that need custom deserialization
pub fn init_app_config() -> anyhow::Result<String> {
    let content = fs::read_to_string("./config/app_config.toml")?;
    let content = substitute_env_vars(content);
    let config_value: Value = content.parse()?;
    let config_map: HashMap<String, Value> = config_value.try_into()?;

    fn insert_config(map: &HashMap<String, Value>, prefix: &str) {
        for (key, value) in map {
            let full_key =
                if prefix.is_empty() { key.clone() } else { format!("{}.{}", prefix, key) };
            match value {
                Value::String(s) => config_manager::set_config(full_key, s.clone()),
                Value::Integer(i) => config_manager::set_config(full_key, i.to_string()),
                Value::Float(f) => config_manager::set_config(full_key, f.to_string()),
                Value::Boolean(b) => config_manager::set_config(full_key, b.to_string()),
                Value::Array(arr) => {
                    let values: Vec<String> =
                        arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
                    config_manager::set_array_config(full_key, values);
                }
                Value::Table(table) => {
                    let sub_map: HashMap<String, Value> = table.clone().into_iter().collect();
                    insert_config(&sub_map, &full_key);
                }
                _ => {}
            }
        }
    }
    insert_config(&config_map, "");

    Ok(content)
}
