use std::fmt;

/// 统一的 S3 服务错误类型
///
/// 定义 S3 服务可能返回的所有错误类型，
/// 提供统一的错误处理接口。
///
/// # 错误类型
///
/// - `AwsError`: AWS SDK 底层错误
/// - `ConfigError`: 配置相关错误
/// - `BucketNotFound`: 桶不存在
/// - `ObjectNotFound`: 对象不存在
/// - `PermissionDenied`: 权限不足
/// - `PresignError`: 预签名 URL 生成失败
/// - `MultipartError`: 分片上传错误
/// - `IoError`: IO 操作错误
/// - `Other`: 其他未分类错误
///
/// # 错误处理示例
///
/// ```rust,no_run
/// use s3_service::S3Error;
///
/// fn handle_error(e: S3Error) {
///     match e {
///         S3Error::BucketNotFound(bucket) => {
///             eprintln!("Bucket {} does not exist", bucket);
///         }
///         S3Error::ObjectNotFound(key) => {
///             eprintln!("Object {} does not exist", key);
///         }
///         _ => eprintln!("Other error: {}", e),
///     }
/// }
/// ```
#[derive(Debug)]
pub enum S3Error {
    /// AWS SDK 错误
    /// 底层 SDK 调用失败时返回
    AwsError(String),

    /// 配置错误
    /// 配置缺失、格式错误或无效
    ConfigError(String),

    /// 桶不存在
    /// 访问不存在的桶时返回
    BucketNotFound(String),

    /// 对象不存在
    /// 访问不存在的对象时返回
    ObjectNotFound(String),

    /// 权限不足
    /// 执行操作时权限不足
    PermissionDenied(String),

    /// 预签名 URL 错误
    /// 创建临时访问 URL 失败
    PresignError(String),

    /// 分片上传错误
    /// 分片上传过程中出错
    MultipartError(String),

    /// IO 错误
    /// 文件读写或其他 IO 操作失败
    IoError(String),

    /// 其他错误
    /// 未分类的错误类型
    Other(String),
}

/// Display trait 实现，提供友好的错误消息
impl fmt::Display for S3Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            S3Error::AwsError(msg) => write!(f, "AWS SDK error: {}", msg),
            S3Error::ConfigError(msg) => write!(f, "S3 config error: {}", msg),
            S3Error::BucketNotFound(bucket) => write!(f, "Bucket not found: {}", bucket),
            S3Error::ObjectNotFound(key) => write!(f, "Object not found: {}", key),
            S3Error::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            S3Error::PresignError(msg) => write!(f, "Pre-signed URL generation failed: {}", msg),
            S3Error::MultipartError(msg) => write!(f, "Multipart upload error: {}", msg),
            S3Error::IoError(msg) => write!(f, "IO error: {}", msg),
            S3Error::Other(msg) => write!(f, "S3 error: {}", msg),
        }
    }
}

/// 标准 Error trait 实现
impl std::error::Error for S3Error {}

/// 从 std::io::Error 转换
///
/// 允许直接使用 ? 操作符将 IO 错误转换为 S3Error
impl From<std::io::Error> for S3Error {
    fn from(err: std::io::Error) -> Self {
        S3Error::IoError(err.to_string())
    }
}

/// 从 anyhow::Error 转换
///
/// 允许将 anyhow 错误转换为 S3Error
impl From<anyhow::Error> for S3Error {
    fn from(err: anyhow::Error) -> Self {
        S3Error::Other(err.to_string())
    }
}
