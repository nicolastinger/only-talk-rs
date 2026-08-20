use std::fs;
use std::net::SocketAddr;

use super::state::ServiceError;

/// QUIC ChatNode 配置
#[derive(Debug, Clone)]
pub struct ChatNodeConfig {
    pub bind_address: SocketAddr,
    pub cert_path: String,
    pub key_path: String,
    pub max_connections: usize,
    pub max_buffer_length: usize,
    pub idle_timeout_secs: u64,
    pub max_concurrent_uni_streams: u8,
    pub server_name: String,
    pub cert_watch_interval_secs: u64,
    pub cert_expiry_warning_days: i64,
    pub cert_expiry_check_interval_secs: u64,
    pub server_index: u32,
    pub node_address: String,
}

impl ChatNodeConfig {
    /// 使用指定地址创建配置,其余字段使用默认值
    pub fn new(bind_address: SocketAddr) -> Self {
        Self {
            bind_address,
            cert_path: "./config/ssl/fullchain.pem".to_string(),
            key_path: "./config/ssl/privkey.pem".to_string(),
            max_connections: 1000,
            max_buffer_length: 10 * 1024 * 1024,
            idle_timeout_secs: 190,
            max_concurrent_uni_streams: 0,
            server_name: "127.0.0.1:4433".to_string(),
            cert_watch_interval_secs: 60,
            cert_expiry_warning_days: 3,
            cert_expiry_check_interval_secs: 3600,
            server_index: 0,
            node_address: "127.0.0.1:4433".to_string(),
        }
    }

    /// 从 TOML 文件读取配置,未配置的字段使用默认值
    pub fn from_toml(path: &str) -> Result<Self, ServiceError> {
        let content = fs::read_to_string(path)
            .map_err(|e| ServiceError::Config(format!("Failed to read config file: {}", e)))?;
        Self::from_toml_str(&content)
    }

    /// 从 TOML 字符串解析配置(调用方必须先完成环境变量替换)
    pub fn from_toml_str(content: &str) -> Result<Self, ServiceError> {
        let config_map: toml::Value = toml::from_str(content)
            .map_err(|e| ServiceError::Config(format!("Failed to parse TOML config: {}", e)))?;

        let quic = config_map.get("quic_server").ok_or_else(|| {
            ServiceError::Config("Missing quic_server config section".to_string())
        })?;

        let addr_str = quic.get("address").and_then(|v| v.as_str()).ok_or_else(|| {
            ServiceError::Config("Missing quic_server.address config item".to_string())
        })?;

        let bind_address: SocketAddr = addr_str
            .parse()
            .map_err(|e| ServiceError::Config(format!("Failed to parse address: {}", e)))?;

        let cert_path = quic
            .get("cert_path")
            .and_then(|v| v.as_str())
            .unwrap_or("./config/ssl/fullchain.pem")
            .to_string();

        let key_path = quic
            .get("key_path")
            .and_then(|v| v.as_str())
            .unwrap_or("./config/ssl/privkey.pem")
            .to_string();

        let server_name = quic
            .get("server_name")
            .and_then(|v| v.as_str())
            .unwrap_or("127.0.0.1:4433")
            .to_string();

        let node_address = quic
            .get("node_address")
            .and_then(|v| v.as_str())
            .unwrap_or("127.0.0.1:4433")
            .to_string();

        let server_index = config_map
            .get("cluster")
            .and_then(|c| c.get("server_index"))
            .and_then(|v| v.as_integer())
            .unwrap_or(0) as u32;

        Ok(Self {
            cert_path,
            key_path,
            server_name,
            node_address,
            server_index,
            ..Self::new(bind_address)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_toml() -> &'static str {
        r#"
[cluster]
server_index = 3

[quic_server]
address = "0.0.0.0:4433"
cert_path = "/certs/fullchain.pem"
key_path = "/certs/privkey.pem"
server_name = "node1:4433"
node_address = "10.0.0.1:4433"
"#
    }

    #[test]
    fn test_new_defaults() {
        let config = ChatNodeConfig::new("127.0.0.1:4433".parse().expect("解析地址失败"));
        assert_eq!(config.bind_address, "127.0.0.1:4433".parse().expect("解析地址失败"));
        assert_eq!(config.max_connections, 1000);
        assert_eq!(config.max_buffer_length, 10 * 1024 * 1024);
        assert_eq!(config.idle_timeout_secs, 190);
        assert_eq!(config.max_concurrent_uni_streams, 0);
        assert_eq!(config.server_name, "127.0.0.1:4433");
        assert_eq!(config.node_address, "127.0.0.1:4433");
        assert_eq!(config.cert_watch_interval_secs, 60);
        assert_eq!(config.cert_expiry_warning_days, 3);
        assert_eq!(config.cert_expiry_check_interval_secs, 3600);
        assert_eq!(config.server_index, 0);
    }

    #[test]
    fn test_from_toml_str_full() {
        let config = ChatNodeConfig::from_toml_str(valid_toml()).expect("解析 TOML 配置失败");
        assert_eq!(config.bind_address, "0.0.0.0:4433".parse().expect("解析地址失败"));
        assert_eq!(config.cert_path, "/certs/fullchain.pem");
        assert_eq!(config.key_path, "/certs/privkey.pem");
        assert_eq!(config.server_name, "node1:4433");
        assert_eq!(config.node_address, "10.0.0.1:4433");
        assert_eq!(config.server_index, 3);
        // 未配置字段使用默认值
        assert_eq!(config.max_connections, 1000);
        assert_eq!(config.idle_timeout_secs, 190);
    }

    #[test]
    fn test_from_toml_str_optional_fields_default() {
        let toml = r#"
[quic_server]
address = "0.0.0.0:8443"
"#;
        let config = ChatNodeConfig::from_toml_str(toml).expect("解析 TOML 配置失败");
        assert_eq!(config.bind_address, "0.0.0.0:8443".parse().expect("解析地址失败"));
        assert_eq!(config.cert_path, "./config/ssl/fullchain.pem");
        assert_eq!(config.key_path, "./config/ssl/privkey.pem");
        assert_eq!(config.server_name, "127.0.0.1:4433");
        assert_eq!(config.node_address, "127.0.0.1:4433");
        assert_eq!(config.server_index, 0);
    }

    #[test]
    fn test_from_toml_str_missing_section() {
        let err = ChatNodeConfig::from_toml_str("[other]\nkey = 1\n").unwrap_err();
        assert!(matches!(err, ServiceError::Config(_)));
    }

    #[test]
    fn test_from_toml_str_missing_address() {
        let toml = "[quic_server]\ncert_path = \"x\"\n";
        let err = ChatNodeConfig::from_toml_str(toml).unwrap_err();
        assert!(matches!(err, ServiceError::Config(_)));
    }

    #[test]
    fn test_from_toml_str_invalid_address() {
        let toml = "[quic_server]\naddress = \"not-an-address\"\n";
        let err = ChatNodeConfig::from_toml_str(toml).unwrap_err();
        assert!(matches!(err, ServiceError::Config(_)));
    }
}
