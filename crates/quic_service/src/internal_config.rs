use std::net::SocketAddr;

use crate::state::ServiceError;

/// 内网 QUIC 服务配置 (无需配置TLS证书)
#[derive(Debug, Clone)]
pub struct InternalQuicConfig {
    pub bind_address: SocketAddr,
    pub server_name: String,
}

impl InternalQuicConfig {
    /// 从 TOML 配置文件读取 internal_quic_server 节
    pub fn from_toml(path: &str) -> Result<Self, ServiceError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ServiceError::Config(format!("读取配置文件失败: {}", e)))?;
        Self::from_toml_str(&content)
    }

    /// 从 TOML 字符串解析配置（调用方需先完成环境变量替换）
    pub fn from_toml_str(content: &str) -> Result<Self, ServiceError> {
        let config_map: toml::Value = toml::from_str(content)
            .map_err(|e| ServiceError::Config(format!("解析TOML配置失败: {}", e)))?;

        let internal = config_map
            .get("internal_quic_server")
            .ok_or_else(|| ServiceError::Config("缺少 internal_quic_server 配置节".to_string()))?;

        let addr_str = internal
            .get("address")
            .and_then(|v| v.as_str())
            .unwrap_or("127.0.0.1:4434");

        let bind_address: SocketAddr = addr_str
            .parse()
            .map_err(|e| ServiceError::Config(format!("解析内网QUIC地址失败: {}", e)))?;

        let server_name = internal
            .get("server_name")
            .and_then(|v| v.as_str())
            .unwrap_or("INTERNAL_SERVER_1")
            .to_string();

        Ok(Self {
            bind_address,
            server_name,
        })
    }
}
