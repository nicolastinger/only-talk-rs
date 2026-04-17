#[warn(semicolon_in_expressions_from_macros)]
#[macro_export]
macro_rules! read_config {
    ($config_map:expr,($($middle_str:expr),*),$final_str:expr) => {
        $config_map.
        $(
          get($middle_str).expect("配置文件中不存在该字段").
        )*
        get($final_str).expect("配置文件中不存在该字段").as_str().expect("配置文件中不存在该字段")
    };
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
            entity::config_manager::get_config(key_str).expect(&format!("配置文件中不存在该字段: {}", key_str))
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
            entity::config_manager::get_array_config(key_str).expect(&format!("配置文件中不存在该字段: {}", key_str))
        }
    };
}

#[warn(semicolon_in_expressions_from_macros)]
#[macro_export]
macro_rules! init_global_config {
    ($config_map:expr) => {
        {
            use entity::config_manager;
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
