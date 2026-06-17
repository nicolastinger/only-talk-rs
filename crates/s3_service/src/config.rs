use std::fmt;

use crate::error::S3Error;

/// S3 storage provider type enum
///
/// Defines the supported S3-compatible storage service types.
/// Currently supports MinIO, Aliyun OSS, and AWS S3.
///
/// # Supported Providers
///
/// - `MinIO`: Open-source object storage service, compatible with AWS S3 API
/// - `AliyunOSS`: Alibaba Cloud object storage service
/// - `AwsS3`: Amazon S3 standard service
///
/// # Example
///
/// ```rust
/// use s3_service::config::S3Provider;
///
/// let provider = S3Provider::from_str("minio").unwrap();
/// assert_eq!(provider, S3Provider::MinIO);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum S3Provider {
    /// MinIO object storage service
    MinIO,
    /// Aliyun OSS object storage service
    AliyunOSS,
    /// AWS S3 standard service
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
    /// Parse storage provider type from string
    ///
    /// Converts a configuration string to an S3Provider enum.
    /// Supports multiple aliases, case-insensitive.
    ///
    /// # Parameters
    ///
    /// - `s`: Provider name string
    ///
    /// # Returns
    ///
    /// Returns the corresponding S3Provider on success, or config error on failure
    ///
    /// # Supported String Formats
    ///
    /// - `"minio"`: MinIO service
    /// - `"aliyun_oss"`, `"aliyun"`, `"oss"`: Aliyun OSS
    /// - `"aws_s3"`, `"aws"`: AWS S3
    ///
    /// # Errors
    ///
    /// Unsupported provider names return a config error
    pub fn from_str(s: &str) -> Result<Self, S3Error> {
        match s.to_lowercase().as_str() {
            "minio" => Ok(S3Provider::MinIO),
            "aliyun_oss" | "aliyun" | "oss" => Ok(S3Provider::AliyunOSS),
            "aws_s3" | "aws" => Ok(S3Provider::AwsS3),
            _ => Err(S3Error::ConfigError(format!(
                "Unsupported S3 provider: {}. Supported: minio, aliyun_oss, aws_s3",
                s
            ))),
        }
    }
}

/// S3 service configuration struct
///
/// Contains all configuration parameters required for S3 service initialization.
/// Supports loading from configuration files or manual construction.
///
/// # Configuration Items
///
/// ## Basic Configuration
/// - `provider`: Storage provider type
/// - `endpoint_url`: S3 service endpoint address
/// - `access_key_id`: Access key ID
/// - `secret_access_key`: Secret access key
/// - `region`: Service region
/// - `default_bucket`: Default bucket name
///
/// ## Advanced Configuration
/// - `force_path_style`: Whether to use path-style access
/// - `enabled`: Whether S3 service is enabled
/// - `presign_expire_seconds`: Pre-signed URL expiration time
/// - `multipart_threshold`: Multipart upload threshold
/// - `multipart_chunk_size`: Chunk size
/// - `max_concurrent_uploads`: Maximum concurrent uploads
///
/// # Example
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
///     group_avatar_bucket: "group-avatar".to_string(),
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
    /// Storage provider type
    /// Specifies which S3-compatible service to use
    pub provider: S3Provider,

    /// S3-compatible endpoint URL
    /// Leave empty for AWS S3, specify for MinIO and OSS
    pub endpoint_url: String,

    /// Access key ID
    /// Username for authentication
    pub access_key_id: String,

    /// Secret access key
    /// Password for authentication
    pub secret_access_key: String,

    /// Service region
    /// e.g., us-east-1, cn-north-1, etc.
    pub region: String,

    /// Default bucket name
    /// Used when no bucket is specified
    pub default_bucket: String,

    /// Chat file preview bucket name
    /// Used to store compressed chat files
    pub chat_file_preview_bucket: String,

    /// Chat file origin bucket name
    /// Used to store original chat files
    pub chat_file_origin_bucket: String,

    /// User avatar bucket name
    /// Used to store user avatar files
    pub user_avatar_bucket: String,

    /// Group avatar bucket name
    /// Used to store group avatar files
    pub group_avatar_bucket: String,

    /// Whether to enable path-style access
    /// true: http://endpoint/bucket/key (required for MinIO)
    /// false: http://bucket.endpoint/key (recommended for AWS S3)
    pub force_path_style: bool,

    /// Whether to enable S3 storage
    /// false may fall back to local storage
    pub enabled: bool,

    /// Pre-signed URL expiration time (seconds)
    /// Validity period for generated temporary access URLs
    pub presign_expire_seconds: u64,

    /// Multipart upload threshold (bytes)
    /// Files exceeding this size use multipart upload
    pub multipart_threshold: i64,

    /// Chunk size (bytes)
    /// Size of each chunk during multipart upload
    pub multipart_chunk_size: i64,

    /// Maximum concurrent uploads
    /// Maximum concurrency during multipart upload
    pub max_concurrent_uploads: usize,
}

impl S3Config {
    /// Load S3 configuration from global config file
    ///
    /// Reads S3-related configuration from the application's config manager.
    /// Supports optional configuration with defaults for missing values.
    ///
    /// # Returns
    ///
    /// Returns a complete S3Config instance, or config error
    ///
    /// # Configuration Items
    ///
    /// ## Required Configuration
    /// - `s3.provider`: Storage provider
    /// - `s3.endpoint`: Service endpoint
    /// - `s3.access_key`: Access key ID
    /// - `s3.secret_key`: Secret access key
    ///
    /// ## Optional Configuration (with defaults)
    /// - `s3.enabled`: Whether enabled, default false
    /// - `s3.region`: Region, default "us-east-1"
    /// - `s3.default_bucket`: Default bucket, default "only-talk-rs"
    /// - `s3.force_path_style`: Path style, default true
    /// - `s3.presign_expire_seconds`: Pre-signed URL expiration, default 3600 seconds
    /// - `s3.multipart_threshold`: Multipart threshold, default 10MB
    /// - `s3.multipart_chunk_size`: Chunk size, default 5MB
    /// - `s3.max_concurrent_uploads`: Concurrency, default 10
    ///
    /// # Errors
    ///
    /// Returns ConfigError for missing required configuration
    pub fn from_global_config() -> Result<Self, S3Error> {
        // Helper function: get config item or return error
        let get_config = |key: &str| -> Result<String, S3Error> {
            common::config_manager::get_config(key)
                .ok_or_else(|| S3Error::ConfigError(format!("Config key not found: {}", key)))
        };

        // Parse storage provider type
        let provider_str = get_config("s3.provider")?;
        let provider = S3Provider::from_str(&provider_str)?;

        // Parse whether S3 service is enabled
        let enabled = get_config("s3.enabled")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);

        // Build configuration object
        Ok(S3Config {
            provider,
            endpoint_url: get_config("s3.endpoint")?,
            access_key_id: get_config("s3.access_key")?,
            secret_access_key: get_config("s3.secret_key")?,
            region: get_config("s3.region").unwrap_or_else(|_| "us-east-1".to_string()),
            default_bucket: get_config("s3.default_bucket")
                .unwrap_or_else(|_| "only-talk-rs".to_string()),
            chat_file_preview_bucket: get_config("s3.chat_file_preview_bucket")
                .unwrap_or_else(|_| "chat-file-preview".to_string()),
            chat_file_origin_bucket: get_config("s3.chat_file_origin_bucket")
                .unwrap_or_else(|_| "chat-file-origin".to_string()),
            user_avatar_bucket: get_config("s3.user_avatar_bucket")
                .unwrap_or_else(|_| "user-avatar".to_string()),
            group_avatar_bucket: get_config("s3.group_avatar_bucket")
                .unwrap_or_else(|_| "group-avatar".to_string()),
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
                .unwrap_or_else(|_| "10485760".to_string()) // 10MB
                .parse::<i64>()
                .unwrap_or(10 * 1024 * 1024),
            multipart_chunk_size: get_config("s3.multipart_chunk_size")
                .unwrap_or_else(|_| "5242880".to_string()) // 5MB
                .parse::<i64>()
                .unwrap_or(5 * 1024 * 1024),
            max_concurrent_uploads: get_config("s3.max_concurrent_uploads")
                .unwrap_or_else(|_| "10".to_string())
                .parse::<usize>()
                .unwrap_or(10),
        })
    }

    /// Create default MinIO configuration
    ///
    /// Provides a pre-configured MinIO configuration instance,
    /// mainly for development and testing environments.
    ///
    /// # Defaults
    ///
    /// - Endpoint: http://xxxx
    /// - Access key: xxxx
    /// - Secret key: xxxxxxx
    /// - Region: us-east-1
    /// - Default bucket: only-talk-rs
    /// - Path style: true (required for MinIO)
    /// - Enabled: true
    /// - Pre-signed expiration: 3600 seconds (1 hour)
    /// - Multipart threshold: 10MB
    /// - Chunk size: 5MB
    /// - Concurrency: 10
    ///
    /// # Security Warning
    ///
    /// Do NOT use default keys in production!
    /// Load actual keys from configuration or environment variables.
    ///
    /// # Example
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
            endpoint_url: "http://xxxx".to_string(),
            access_key_id: "xxxx".to_string(),
            secret_access_key: "xxxxx".to_string(),
            region: "us-east-1".to_string(),
            default_bucket: "only-talk-rs".to_string(),
            chat_file_preview_bucket: "chat-file-preview".to_string(),
            chat_file_origin_bucket: "chat-file-origin".to_string(),
            user_avatar_bucket: "user-avatar".to_string(),
            group_avatar_bucket: "group-avatar".to_string(),
            force_path_style: true, // MinIO requires path-style
            enabled: true,
            presign_expire_seconds: 3600,          // 1 hour
            multipart_threshold: 10 * 1024 * 1024, // 10MB
            multipart_chunk_size: 5 * 1024 * 1024, // 5MB
            max_concurrent_uploads: 10,
        }
    }
}
