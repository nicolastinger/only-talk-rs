use std::collections::HashMap;
use std::fs;

use ::tracing::log;
use toml::Value;

pub mod config_manager;
pub mod config_str;
pub mod macros;
pub mod state;
pub mod tracing;
pub mod utils;

// 重导出
// 重导出 entity 模型,外部 crate 只需依赖 core 即可访问数据库实体
pub use entity::models;
pub use utils::fatal_error::{fatal_panic, fatal_panic_async};
pub use utils::internal_quic_client::send_internal_quic_msg;
pub use utils::redis_utils::{init_redis, verify_redis};
pub use utils::server_count_sync::{SERVER_COUNT, get_server_count, start_server_count_sync};
pub use utils::sql_utils::init_sql_pool;

/// 替换字符串中的环境变量占位符 ${VAR_NAME}
/// 最多迭代 100 次,防止恶意配置导致死循环
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

/// 读取应用配置: 替换环境变量、解析 TOML、填充全局 DashMap
/// 返回替换后的 TOML 字符串,供需要自定义反序列化的调用方使用
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
