use std::fs;
use std::net::SocketAddr;

use crate::state::ServiceError;

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
}

impl ChatNodeConfig {
    /// 使用指定地址创建配置，其余字段使用默认值
    pub fn new(bind_address: SocketAddr) -> Self {
        Self {
            bind_address,
            cert_path: "./config/ssl/fullchain.pem".to_string(),
            key_path: "./config/ssl/privkey.pem".to_string(),
            max_connections: 1000,
            max_buffer_length: 10 * 1024 * 1024,
            idle_timeout_secs: 190,
            max_concurrent_uni_streams: 0,
            server_name: "SERVER_1".to_string(),
            cert_watch_interval_secs: 60,
            cert_expiry_warning_days: 3,
            cert_expiry_check_interval_secs: 3600,
        }
    }

    /// 从 TOML 配置文件读取配置，未配置的字段使用默认值
    pub fn from_toml(path: &str) -> Result<Self, ServiceError> {
        let content = fs::read_to_string(path)
            .map_err(|e| ServiceError::Config(format!("读取配置文件失败: {}", e)))?;
        let config_map: toml::Value = toml::from_str(&content)
            .map_err(|e| ServiceError::Config(format!("解析TOML配置失败: {}", e)))?;

        let quic = config_map
            .get("quic_server")
            .ok_or_else(|| ServiceError::Config("缺少 quic_server 配置节".to_string()))?;

        let addr_str = quic
            .get("address")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServiceError::Config("缺少 quic_server.address 配置项".to_string()))?;

        let bind_address: SocketAddr = addr_str
            .parse()
            .map_err(|e| ServiceError::Config(format!("解析地址失败: {}", e)))?;

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
            .unwrap_or("SERVER_1")
            .to_string();

        Ok(Self {
            cert_path,
            key_path,
            server_name,
            ..Self::new(bind_address)
        })
    }
}
