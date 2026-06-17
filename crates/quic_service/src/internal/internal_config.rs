use std::net::SocketAddr;

use crate::external::state::ServiceError;

/// Internal QUIC service configuration (no TLS cert required)
#[derive(Debug, Clone)]
pub struct InternalQuicConfig {
    pub bind_address: SocketAddr,
    pub server_name: String,
    pub server_index: u32,
    pub node_address: String,
}

impl InternalQuicConfig {
    /// Read internal_quic_server section from TOML config file
    pub fn from_toml(path: &str) -> Result<Self, ServiceError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ServiceError::Config(format!("Failed to read config file: {}", e)))?;
        Self::from_toml_str(&content)
    }

    /// Parse config from TOML string (caller must complete env var substitution first)
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
