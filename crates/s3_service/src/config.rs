use std::fmt;

use crate::error::S3Error;

/// S3存储提供者类型枚举
///
/// 定义支持的S3兼容存储服务类型。
/// 目前支持MinIO、阿里云OSS和AWS S3三种。
///
/// # 支持的提供者
///
/// - `MinIO`: 开源的对象存储服务,兼容AWS S3 API
/// - `AliyunOSS`: 阿里云对象存储服务
/// - `AwsS3`: Amazon S3标准服务
///
/// # 示例
///
/// ```rust
/// use s3_service::config::S3Provider;
///
/// let provider = S3Provider::from_str("minio").unwrap();
/// assert_eq!(provider, S3Provider::MinIO);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum S3Provider {
    /// MinIO对象存储服务
    MinIO,
    /// 阿里云OSS对象存储服务
    AliyunOSS,
    /// AWS S3标准服务
    AwsS3,
}

impl fmt::Display for S3Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            S3Provider::MinIO => write!(f, "minio"),
            S3Provider::AliyunOSS => write!(f, "aliyun_oss"),
            S3Provider::AwsS3 => write!(f, "aws_s3"),
        }
    }
}

impl S3Provider {
    /// 从字符串解析存储提供者类型
    ///
    /// 将配置文件中的字符串转换为S3Provider枚举。
    /// 支持多种别名,不区分大小写。
    ///
    /// # 参数
    ///
    /// - `s`: 提供者名称字符串
    ///
    /// # 返回值
    ///
    /// 解析成功返回对应的S3Provider,失败返回配置错误
    ///
    /// # 支持的字符串格式
    ///
    /// - `"minio"`: MinIO服务
    /// - `"aliyun_oss"`, `"aliyun"`, `"oss"`: 阿里云OSS
    /// - `"aws_s3"`, `"aws"`: AWS S3
    ///
    /// # 错误
    ///
    /// 不支持的提供者名称会返回配置错误
    pub fn from_str(s: &str) -> Result<Self, S3Error> {
        match s.to_lowercase().as_str() {
            "minio" => Ok(S3Provider::MinIO),
            "aliyun_oss" | "aliyun" | "oss" => Ok(S3Provider::AliyunOSS),
            "aws_s3" | "aws" => Ok(S3Provider::AwsS3),
            _ => Err(S3Error::ConfigError(format!(
                "不支持的S3提供者: {}，支持: minio, aliyun_oss, aws_s3",
                s
            ))),
        }
    }
}

/// S3服务配置结构体
///
/// 包含所有S3服务初始化所需的配置参数。
/// 支持从配置文件加载或手动构建。
///
/// # 配置项说明
///
/// ## 基础配置
/// - `provider`: 存储提供者类型
/// - `endpoint_url`: S3服务端点地址
/// - `access_key_id`: 访问密钥ID
/// - `secret_access_key`: 访问密钥
/// - `region`: 服务区域
/// - `default_bucket`: 默认存储桶名称
///
/// ## 高级配置
/// - `force_path_style`: 是否使用路径风格访问
/// - `enabled`: 是否启用S3服务
/// - `presign_expire_seconds`: 预签名URL过期时间
/// - `multipart_threshold`: 分片上传阈值
/// - `multipart_chunk_size`: 分片大小
/// - `max_concurrent_uploads`: 最大并发上传数
///
/// # 示例
///
/// ```rust
/// use s3_service::config::{S3Config, S3Provider};
///
/// let config = S3Config {
///     provider: S3Provider::MinIO,
///     endpoint_url: "http://localhost:9000".to_string(),
///     access_key_id: "minioadmin".to_string(),
///     secret_access_key: "minioadmin".to_string(),
///     region: "us-east-1".to_string(),
///     default_bucket: "my-bucket".to_string(),
///     chat_file_preview_bucket: "chat-file-preview".to_string(),
///     chat_file_origin_bucket: "chat-file-origin".to_string(),
///     user_avatar_bucket: "user-avatar".to_string(),
///     force_path_style: true,
///     enabled: true,
///     presign_expire_seconds: 3600,
///     multipart_threshold: 10 * 1024 * 1024,
///     multipart_chunk_size: 5 * 1024 * 1024,
///     max_concurrent_uploads: 10,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct S3Config {
    /// 存储提供者类型
    /// 指定使用哪个S3兼容服务
    pub provider: S3Provider,
    
    /// S3兼容端点URL
    /// AWS S3可留空,MinIO和OSS需要指定具体地址
    pub endpoint_url: String,
    
    /// 访问密钥ID
    /// 用于身份认证的用户名
    pub access_key_id: String,
    
    /// 访问密钥
    /// 用于身份认证的密码
    pub secret_access_key: String,
    
    /// 服务区域
    /// 如: us-east-1, cn-north-1等
    pub region: String,
    
    /// 默认存储桶名称
    /// 未指定桶名时使用此桶
    pub default_bucket: String,
    
    /// 聊天文件预览桶名称
    /// 用于存储压缩后的聊天文件
    pub chat_file_preview_bucket: String,
    
    /// 聊天文件原文件桶名称
    /// 用于存储原始聊天文件
    pub chat_file_origin_bucket: String,
    
    /// 用户头像桶名称
    /// 用于存储用户头像文件
    pub user_avatar_bucket: String,
    
    /// 是否启用路径风格访问
    /// MinIO必须为true,AWS S3推荐false
    /// true: http://endpoint/bucket/key
    /// false: http://bucket.endpoint/key
    pub force_path_style: bool,
    
    /// 是否启用S3存储
    /// false时可能回退到本地存储
    pub enabled: bool,
    
    /// 预签名URL过期时间(秒)
    /// 生成的临时访问URL的有效期
    pub presign_expire_seconds: u64,
    
    /// 分片上传阈值(字节)
    /// 文件大小超过此值时启用分片上传
    pub multipart_threshold: i64,
    
    /// 分片大小(字节)
    /// 分片上传时每个分片的大小
    pub multipart_chunk_size: i64,
    
    /// 最大并发上传数
    /// 分片上传时的最大并发数
    pub max_concurrent_uploads: usize,
}

impl S3Config {
    /// 从全局配置文件加载S3配置
    ///
    /// 从应用的配置管理器中读取S3相关配置项。
    /// 支持可选配置,缺失时使用默认值。
    ///
    /// # 返回值
    ///
    /// 返回完整的S3Config实例,或配置错误
    ///
    /// # 配置项说明
    ///
    /// ## 必需配置
    /// - `s3.provider`: 存储提供者
    /// - `s3.endpoint`: 服务端点
    /// - `s3.access_key`: 访问密钥ID
    /// - `s3.secret_key`: 访问密钥
    ///
    /// ## 可选配置(有默认值)
    /// - `s3.enabled`: 是否启用,默认false
    /// - `s3.region`: 区域,默认"us-east-1"
    /// - `s3.default_bucket`: 默认桶,默认"rust-my-app"
    /// - `s3.force_path_style`: 路径风格,默认true
    /// - `s3.presign_expire_seconds`: 预签名过期时间,默认3600秒
    /// - `s3.multipart_threshold`: 分片阈值,默认10MB
    /// - `s3.multipart_chunk_size`: 分片大小,默认5MB
    /// - `s3.max_concurrent_uploads`: 并发数,默认10
    ///
    /// # 错误
    ///
    /// 必需配置缺失时返回ConfigError
    pub fn from_global_config() -> Result<Self, S3Error> {
        // 辅助函数: 获取配置项或返回错误
        let get_config = |key: &str| -> Result<String, S3Error> {
            common::config_manager::get_config(key).ok_or_else(|| {
                S3Error::ConfigError(format!("配置文件中不存在该字段: {}", key))
            })
        };

        // 解析存储提供者类型
        let provider_str = get_config("s3.provider")?;
        let provider = S3Provider::from_str(&provider_str)?;

        // 解析是否启用S3服务
        let enabled = get_config("s3.enabled")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);

        // 构建配置对象
        Ok(S3Config {
            provider,
            endpoint_url: get_config("s3.endpoint")?,
            access_key_id: get_config("s3.access_key")?,
            secret_access_key: get_config("s3.secret_key")?,
            region: get_config("s3.region").unwrap_or_else(|_| "us-east-1".to_string()),
            default_bucket: get_config("s3.default_bucket")
                .unwrap_or_else(|_| "rust-my-app".to_string()),
            chat_file_preview_bucket: get_config("s3.chat_file_preview_bucket")
                .unwrap_or_else(|_| "chat-file-preview".to_string()),
            chat_file_origin_bucket: get_config("s3.chat_file_origin_bucket")
                .unwrap_or_else(|_| "chat-file-origin".to_string()),
            user_avatar_bucket: get_config("s3.user_avatar_bucket")
                .unwrap_or_else(|_| "user-avatar".to_string()),
            force_path_style: get_config("s3.force_path_style")
                .unwrap_or_else(|_| "true".to_string())
                .parse::<bool>()
                .unwrap_or(true),
            enabled,
            presign_expire_seconds: get_config("s3.presign_expire_seconds")
                .unwrap_or_else(|_| "3600".to_string())
                .parse::<u64>()
                .unwrap_or(3600),
            multipart_threshold: get_config("s3.multipart_threshold")
                .unwrap_or_else(|_| "10485760".to_string())  // 10MB
                .parse::<i64>()
                .unwrap_or(10 * 1024 * 1024),
            multipart_chunk_size: get_config("s3.multipart_chunk_size")
                .unwrap_or_else(|_| "5242880".to_string())   // 5MB
                .parse::<i64>()
                .unwrap_or(5 * 1024 * 1024),
            max_concurrent_uploads: get_config("s3.max_concurrent_uploads")
                .unwrap_or_else(|_| "10".to_string())
                .parse::<usize>()
                .unwrap_or(10),
        })
    }

    /// 创建默认的MinIO配置
    ///
    /// 提供一个预配置的MinIO配置实例,
    /// 主要用于开发和测试环境。
    ///
    /// # 默认值
    ///
    /// - 端点: http://101.33.75.40:19000
    /// - 访问密钥: minioadmin
    /// - 区域: us-east-1
    /// - 默认桶: rust-my-app
    /// - 路径风格: true (MinIO必需)
    /// - 启用状态: true
    /// - 预签名过期: 3600秒(1小时)
    /// - 分片阈值: 10MB
    /// - 分片大小: 5MB
    /// - 并发数: 10
    ///
    /// # 安全警告
    ///
    /// 生产环境请勿使用默认密钥!
    /// 请从配置文件或环境变量加载实际密钥。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use s3_service::S3Config;
    ///
    /// let config = S3Config::default_minio();
    /// assert!(config.enabled);
    /// ```
    pub fn default_minio() -> Self {
        S3Config {
            provider: S3Provider::MinIO,
            endpoint_url: "http://101.33.75.40:19000".to_string(),
            access_key_id: "REDACTED_S3_ACCESS_KEY".to_string(),
            secret_access_key: "REDACTED_S3_SECRET_KEY_V2".to_string(),
            region: "us-east-1".to_string(),
            default_bucket: "rust-my-app".to_string(),
            chat_file_preview_bucket: "chat-file-preview".to_string(),
            chat_file_origin_bucket: "chat-file-origin".to_string(),
            user_avatar_bucket: "user-avatar".to_string(),
            force_path_style: true,      // MinIO必须使用路径风格
            enabled: true,
            presign_expire_seconds: 3600, // 1小时
            multipart_threshold: 10 * 1024 * 1024,  // 10MB
            multipart_chunk_size: 5 * 1024 * 1024,   // 5MB
            max_concurrent_uploads: 10,
        }
    }
}
