use dashmap::DashMap;
use lazy_static::lazy_static;
use std::sync::Arc;

lazy_static! {
    pub static ref GLOBAL_CONFIG: Arc<DashMap<String, String>> = Arc::new(DashMap::new());
}

pub fn set_config(key: String, value: String) {
    GLOBAL_CONFIG.insert(key, value);
}

pub fn get_config(key: &str) -> Option<String> {
    GLOBAL_CONFIG.get(key).map(|v| v.value().clone())
}

pub fn remove_config(key: &str) -> Option<String> {
    GLOBAL_CONFIG.remove(key).map(|(_, v)| v)
}

pub fn clear_config() {
    GLOBAL_CONFIG.clear();
}

/// Set array-type config, comma-separated
pub fn set_array_config(key: String, values: Vec<String>) {
    let value = values.join(",");
    GLOBAL_CONFIG.insert(key, value);
}

/// Get array-type config
pub fn get_array_config(key: &str) -> Option<Vec<String>> {
    GLOBAL_CONFIG.get(key).map(|v| {
        v.value().split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
    })
}
