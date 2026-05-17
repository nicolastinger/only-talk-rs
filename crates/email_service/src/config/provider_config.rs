use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProviderConfig {
    Aliyun(AliyunConfig),
    Tencent(TencentConfig),
    AwsSes(AwsSesConfig),
    Smtp(SmtpConfig),
}

impl ProviderConfig {
    pub fn provider_name(&self) -> &'static str {
        match self {
            ProviderConfig::Aliyun(_) => "aliyun",
            ProviderConfig::Tencent(_) => "tencent",
            ProviderConfig::AwsSes(_) => "aws_ses",
            ProviderConfig::Smtp(_) => "smtp",
        }
    }

    pub fn is_enabled(&self) -> bool {
        match self {
            ProviderConfig::Aliyun(c) => c.enabled,
            ProviderConfig::Tencent(c) => c.enabled,
            ProviderConfig::AwsSes(c) => c.enabled,
            ProviderConfig::Smtp(c) => c.enabled,
        }
    }

    pub fn priority(&self) -> u32 {
        match self {
            ProviderConfig::Aliyun(c) => c.priority,
            ProviderConfig::Tencent(c) => c.priority,
            ProviderConfig::AwsSes(c) => c.priority,
            ProviderConfig::Smtp(c) => c.priority,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AliyunConfig {
    pub enabled: bool,
    pub priority: u32,
    pub access_key_id: String,
    pub access_key_secret: String,
    pub region_id: String,
    pub account_name: String,
    pub from_alias: Option<String>,
    pub endpoint: Option<String>,
    pub timeout_ms: u64,
}

impl Default for AliyunConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            priority: 100,
            access_key_id: String::new(),
            access_key_secret: String::new(),
            region_id: "cn-hangzhou".to_string(),
            account_name: String::new(),
            from_alias: None,
            endpoint: None,
            timeout_ms: 30000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TencentConfig {
    pub enabled: bool,
    pub priority: u32,
    pub secret_id: String,
    pub secret_key: String,
    pub region: String,
    pub sender: String,
    pub from_alias: Option<String>,
    pub timeout_ms: u64,
}

impl Default for TencentConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            priority: 100,
            secret_id: String::new(),
            secret_key: String::new(),
            region: "ap-guangzhou".to_string(),
            sender: String::new(),
            from_alias: None,
            timeout_ms: 30000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwsSesConfig {
    pub enabled: bool,
    pub priority: u32,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub region: String,
    pub from_email: String,
    pub from_alias: Option<String>,
    pub configuration_set: Option<String>,
    pub timeout_ms: u64,
}

impl Default for AwsSesConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            priority: 100,
            access_key_id: String::new(),
            secret_access_key: String::new(),
            region: "us-east-1".to_string(),
            from_email: String::new(),
            from_alias: None,
            configuration_set: None,
            timeout_ms: 30000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtpConfig {
    pub enabled: bool,
    pub priority: u32,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from_email: String,
    pub from_alias: Option<String>,
    pub use_tls: bool,
    pub use_starttls: bool,
    pub timeout_ms: u64,
}

impl Default for SmtpConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            priority: 50,
            host: String::new(),
            port: 587,
            username: String::new(),
            password: String::new(),
            from_email: String::new(),
            from_alias: None,
            use_tls: true,
            use_starttls: true,
            timeout_ms: 30000,
        }
    }
}
