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
