use std::net::SocketAddr;

use crate::external::state::ServiceError;

/// 内部 QUIC 服务配置(无需 TLS 证书)
#[derive(Debug, Clone)]
pub struct InternalQuicConfig {
    pub bind_address: SocketAddr,
    pub server_name: String,
    pub server_index: u32,
    pub node_address: String,
}

impl InternalQuicConfig {
    /// 从 TOML 配置文件中读取 internal_quic_server 配置段
    pub fn from_toml(path: &str) -> Result<Self, ServiceError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ServiceError::Config(format!("Failed to read config file: {}", e)))?;
        Self::from_toml_str(&content)
    }

    /// 从 TOML 字符串解析配置(调用方必须先完成环境变量替换)
    pub fn from_toml_str(content: &str) -> Result<Self, ServiceError> {
        let config_map: toml::Value = toml::from_str(content)
            .map_err(|e| ServiceError::Config(format!("Failed to parse TOML config: {}", e)))?;

        let internal = config_map.get("internal_quic_server").ok_or_else(|| {
            ServiceError::Config("Missing internal_quic_server config section".to_string())
        })?;

        let addr_str = internal.get("address").and_then(|v| v.as_str()).unwrap_or("127.0.0.1:4434");

        let bind_address: SocketAddr = addr_str.parse().map_err(|e| {
            ServiceError::Config(format!("Failed to parse internal QUIC address: {}", e))
        })?;

        let server_name = internal
            .get("server_name")
            .and_then(|v| v.as_str())
            .unwrap_or("INTERNAL_SERVER_1")
            .to_string();

        let node_address = internal
            .get("node_address")
            .and_then(|v| v.as_str())
            .unwrap_or("127.0.0.1:4434")
            .to_string();

        let server_index = config_map
            .get("cluster")
            .and_then(|c| c.get("server_index"))
            .and_then(|v| v.as_integer())
            .unwrap_or(0) as u32;

        Ok(Self { bind_address, server_name, server_index, node_address })
    }
}

#[cfg(test)]
// 测试代码中直接使用 unwrap 作为断言失败手段是惯例,此处豁免生产代码的 unwrap 禁令
#[allow(clippy::unwrap_used, clippy::disallowed_methods)]
mod tests {
    use super::*;

    fn valid_toml() -> &'static str {
        r#"
[cluster]
server_index = 2

[internal_quic_server]
address = "127.0.0.1:4434"
server_name = "INTERNAL_1"
node_address = "10.0.0.2:4434"
"#
    }

    #[test]
    fn test_from_toml_str_full() {
        let config = InternalQuicConfig::from_toml_str(valid_toml()).unwrap();
        assert_eq!(config.bind_address, "127.0.0.1:4434".parse().unwrap());
        assert_eq!(config.server_name, "INTERNAL_1");
        assert_eq!(config.node_address, "10.0.0.2:4434");
        assert_eq!(config.server_index, 2);
    }

    #[test]
    fn test_from_toml_str_optional_fields_default() {
        let toml = "[internal_quic_server]\naddress = \"127.0.0.1:9444\"\n";
        let config = InternalQuicConfig::from_toml_str(toml).unwrap();
        assert_eq!(config.bind_address, "127.0.0.1:9444".parse().unwrap());
        assert_eq!(config.server_name, "INTERNAL_SERVER_1");
        assert_eq!(config.node_address, "127.0.0.1:4434");
        assert_eq!(config.server_index, 0);
    }

    #[test]
    fn test_from_toml_str_missing_section() {
        let err = InternalQuicConfig::from_toml_str("[x]\n").unwrap_err();
        assert!(matches!(err, ServiceError::Config(_)));
    }

    #[test]
    fn test_from_toml_str_invalid_address() {
        let toml = "[internal_quic_server]\naddress = \"bad\"\n";
        let err = InternalQuicConfig::from_toml_str(toml).unwrap_err();
        assert!(matches!(err, ServiceError::Config(_)));
    }
}
