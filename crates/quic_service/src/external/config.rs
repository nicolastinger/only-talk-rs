use std::fs;
use std::net::SocketAddr;

use super::state::ServiceError;

/// QUIC ChatNode configuration
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
    /// Create config with specified address, other fields use defaults
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

    /// Read config from TOML file, unconfigured fields use defaults
    pub fn from_toml(path: &str) -> Result<Self, ServiceError> {
        let content = fs::read_to_string(path)
            .map_err(|e| ServiceError::Config(format!("Failed to read config file: {}", e)))?;
        Self::from_toml_str(&content)
    }

    /// Parse config from TOML string (caller must complete env var substitution first)
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
