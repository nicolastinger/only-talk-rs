//! common 配置读取宏(read_config / read_global_config / read_global_array_config)测试

use common::{read_config, read_global_array_config, read_global_config};

/// 构造根 TOML: { db = { url = "postgres://x" } }
fn toml_root() -> toml::Value {
    let mut db = toml::map::Map::new();
    db.insert("url".to_string(), toml::Value::String("postgres://x".to_string()));
    let mut root = toml::map::Map::new();
    root.insert("db".to_string(), toml::Value::Table(db));
    toml::Value::Table(root)
}

mod read_config {
    use super::*;

    #[test]
    fn extracts_nested_value() {
        let root = toml_root();
        let result: Result<String, anyhow::Error> = read_config!(&root, ("db"), "url");
        assert_eq!(result.expect("read_config 读取配置失败"), "postgres://x");
    }

    #[test]
    fn missing_final_field_errors() {
        let root = toml_root();
        let result: Result<String, anyhow::Error> = read_config!(&root, ("db"), "nope");
        assert!(result.is_err());
    }

    #[test]
    fn missing_middle_field_errors() {
        let root = toml_root();
        let result: Result<String, anyhow::Error> = read_config!(&root, ("missing"), "url");
        assert!(result.is_err());
    }
}

// read_global_config! / read_global_array_config! 内部以 `?` 结束,
// 成功时直接求值为对应值,失败时向调用方返回 Err,因此需经 Result 返回函数包裹测试。

mod read_global_config {
    use super::*;

    fn read_test_port() -> anyhow::Result<String> {
        Ok(read_global_config!("test", "mac", "port"))
    }

    fn read_missing() -> anyhow::Result<String> {
        Ok(read_global_config!("no", "such", "key"))
    }

    #[test]
    fn joins_keys_and_reads_from_config() {
        common::config_manager::set_config("test.mac.port".to_string(), "8080".to_string());
        assert_eq!(read_test_port().expect("read_global_config 读取配置失败"), "8080");
        common::config_manager::remove_config("test.mac.port");
    }

    #[test]
    fn missing_key_errors() {
        assert!(read_missing().is_err());
    }
}

mod read_global_array_config {
    use super::*;

    fn read_test_list() -> anyhow::Result<Vec<String>> {
        Ok(read_global_array_config!("test", "mac", "list"))
    }

    fn read_missing() -> anyhow::Result<Vec<String>> {
        Ok(read_global_array_config!("no", "such", "key"))
    }

    #[test]
    fn reads_array_from_config() {
        common::config_manager::set_array_config(
            "test.mac.list".to_string(),
            vec!["a".to_string(), "b".to_string()],
        );
        assert_eq!(
            read_test_list().expect("read_global_array_config 读取配置失败"),
            vec!["a".to_string(), "b".to_string()]
        );
        common::config_manager::remove_config("test.mac.list");
    }

    #[test]
    fn missing_array_key_errors() {
        assert!(read_missing().is_err());
    }
}
