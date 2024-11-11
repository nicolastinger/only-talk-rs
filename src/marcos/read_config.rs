#[macro_export]
macro_rules! read_config {
    ($config_map:expr,($($middle_str:expr),*),$final_str:expr) => {
        $config_map.
        $(
          get($middle_str).unwrap().
        )*
        get($final_str).unwrap().as_str().unwrap();
    };
}