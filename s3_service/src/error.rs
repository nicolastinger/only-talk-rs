use std::fmt;

/// S3服务统一错误类型
///
/// 定义了S3服务可能返回的所有错误类型,
/// 提供统一的错误处理接口。
///
/// # 错误类型说明
///
/// - `AwsError`: AWS SDK底层错误
/// - `ConfigError`: 配置相关错误
/// - `BucketNotFound`: 存储桶不存在
/// - `ObjectNotFound`: 对象不存在
/// - `PermissionDenied`: 权限不足
/// - `PresignError`: 预签名URL生成失败
/// - `MultipartError`: 分片上传错误
/// - `IoError`: IO操作错误
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
///             eprintln!("存储桶 {} 不存在", bucket);
///         }
///         S3Error::ObjectNotFound(key) => {
///             eprintln!("对象 {} 不存在", key);
///         }
///         _ => eprintln!("其他错误: {}", e),
///     }
/// }
/// ```
#[derive(Debug)]
pub enum S3Error {
    /// AWS SDK错误
    /// 底层SDK调用失败时返回的错误
    AwsError(String),
    
    /// 配置错误
    /// 配置项缺失、格式错误或值无效
    ConfigError(String),
    
    /// 存储桶不存在
    /// 访问不存在的存储桶时返回
    BucketNotFound(String),
    
    /// 对象不存在
    /// 访问不存在的对象时返回
    ObjectNotFound(String),
    
    /// 权限不足
    /// 缺少执行操作的权限
    PermissionDenied(String),
    
    /// 预签名URL生成失败
    /// 创建临时访问URL时出错
    PresignError(String),
    
    /// 分片上传错误
    /// 分片上传过程中发生的错误
    MultipartError(String),
    
    /// IO错误
    /// 文件读写等IO操作失败
    IoError(String),
    
    /// 其他错误
    /// 未分类的其他类型错误
    Other(String),
}

/// 实现Display trait,提供友好的错误消息
impl fmt::Display for S3Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            S3Error::AwsError(msg) => write!(f, "AWS SDK错误: {}", msg),
            S3Error::ConfigError(msg) => write!(f, "S3配置错误: {}", msg),
            S3Error::BucketNotFound(bucket) => write!(f, "存储桶不存在: {}", bucket),
            S3Error::ObjectNotFound(key) => write!(f, "对象不存在: {}", key),
            S3Error::PermissionDenied(msg) => write!(f, "权限不足: {}", msg),
            S3Error::PresignError(msg) => write!(f, "预签名URL生成失败: {}", msg),
            S3Error::MultipartError(msg) => write!(f, "分片上传错误: {}", msg),
            S3Error::IoError(msg) => write!(f, "IO错误: {}", msg),
            S3Error::Other(msg) => write!(f, "S3错误: {}", msg),
        }
    }
}

/// 实现标准Error trait
impl std::error::Error for S3Error {}

/// 从std::io::Error转换
///
/// 允许直接使用?操作符将IO错误转换为S3Error
impl From<std::io::Error> for S3Error {
    fn from(err: std::io::Error) -> Self {
        S3Error::IoError(err.to_string())
    }
}

/// 从anyhow::Error转换
///
/// 允许将anyhow错误转换为S3Error
impl From<anyhow::Error> for S3Error {
    fn from(err: anyhow::Error) -> Self {
        S3Error::Other(err.to_string())
    }
}
