#[warn(semicolon_in_expressions_from_macros)]
#[macro_export]
macro_rules! read_config {
    ($config_map:expr,($($middle_str:expr),*),$final_str:expr) => {{
        let mut value: Option<&toml::Value> = Some($config_map);
        $(
            value = value.and_then(|v| v.get($middle_str));
        )*
        match value.and_then(|v| v.get($final_str)).and_then(|v| v.as_str()) {
            Some(s) => Ok(s.to_string()),
            None => Err(anyhow::anyhow!("配置文件中不存在该字段: {}", $final_str)),
        }
    }};
}

#[warn(semicolon_in_expressions_from_macros)]
#[macro_export]
macro_rules! read_global_config {
    ($($key:expr),+) => {
        {
            let mut full_key = String::new();
            $(
                full_key.push_str($key);
                full_key.push('.');
            )+
            let key_str = full_key.trim_end_matches('.');
            match common::config_manager::get_config(key_str) {
                Some(v) => Ok(v),
                None => Err(anyhow::anyhow!("配置文件中不存在该字段: {}", key_str)),
            }?
        }
    };
}

#[warn(semicolon_in_expressions_from_macros)]
#[macro_export]
macro_rules! read_global_array_config {
    ($($key:expr),+) => {
        {
            let mut full_key = String::new();
            $(
                full_key.push_str($key);
                full_key.push('.');
            )+
            let key_str = full_key.trim_end_matches('.');
            match common::config_manager::get_array_config(key_str) {
                Some(v) => Ok(v),
                None => Err(anyhow::anyhow!("配置文件中不存在该数组字段: {}", key_str)),
            }?
        }
    };
}

#[warn(semicolon_in_expressions_from_macros)]
#[macro_export]
macro_rules! init_global_config {
    ($config_map:expr) => {
        {
            use common::config_manager;
            fn insert_config(map: &std::collections::HashMap<String, toml::Value>, prefix: &str) {
                for (key, value) in map {
                    let full_key = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", prefix, key)
                    };
                    match value {
                        toml::Value::String(s) => {
                            config_manager::set_config(full_key, s.clone());
                        }
                        toml::Value::Integer(i) => {
                            config_manager::set_config(full_key, i.to_string());
                        }
                        toml::Value::Float(f) => {
                            config_manager::set_config(full_key, f.to_string());
                        }
                        toml::Value::Boolean(b) => {
                            config_manager::set_config(full_key, b.to_string());
                        }
                        toml::Value::Array(arr) => {
                            let values: Vec<String> = arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect();
                            config_manager::set_array_config(full_key, values);
                        }
                        toml::Value::Table(table) => {
                            let sub_map: std::collections::HashMap<String, toml::Value> = table.clone().into_iter().collect();
                            insert_config(&sub_map, &full_key);
                        }
                        _ => {}
                    }
                }
            }
            insert_config($config_map, "");
        }
    };
}
