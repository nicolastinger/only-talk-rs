//! 存储桶操作模块
//!
//! 提供存储桶的创建、删除、列举、权限配置等操作。

use aws_sdk_s3::types::Bucket;
use aws_sdk_s3::primitives::DateTime;

use crate::client::S3Client;
use crate::error::S3Error;

/// 创建存储桶
///
/// 创建一个新的S3存储桶。
///
/// # 参数
///
/// - `client`: S3客户端实例
/// - `bucket`: 存储桶名称
///
/// # 注意事项
///
/// - 桶名必须全局唯一
/// - 桶名需符合S3命名规范
/// - 不同区域可能有不同限制
///
/// # 示例
///
/// ```rust,no_run
/// use s3_service::operations::create_bucket;
///
/// async fn example(client: &s3_service::S3Client) -> Result<(), s3_service::S3Error> {
///     create_bucket(client, "my-new-bucket").await
/// }
/// ```
pub async fn create_bucket(
    client: &S3Client,
    bucket: &str,
) -> Result<(), S3Error> {
    client
        .inner
        .create_bucket()
        .bucket(bucket)
        .send()
        .await
        .map_err(|e| S3Error::AwsError(format!("创建存储桶失败: {}", e)))?;
    Ok(())
}

/// 删除存储桶
///
/// 删除一个空的S3存储桶。
/// 桶必须为空才能删除,否则会失败。
///
/// # 参数
///
/// - `client`: S3客户端实例
/// - `bucket`: 要删除的存储桶名称
///
/// # 错误
///
/// - 桶不为空
/// - 桶不存在
/// - 权限不足
pub async fn delete_bucket(
    client: &S3Client,
    bucket: &str,
) -> Result<(), S3Error> {
    client
        .inner
        .delete_bucket()
        .bucket(bucket)
        .send()
        .await
        .map_err(|e| S3Error::AwsError(format!("删除存储桶失败: {}", e)))?;
    Ok(())
}

/// 列出所有存储桶
///
/// 列出当前账户下的所有存储桶。
///
/// # 参数
///
/// - `client`: S3客户端实例
///
/// # 返回值
///
/// 返回存储桶信息列表
pub async fn list_buckets(client: &S3Client) -> Result<Vec<BucketInfo>, S3Error> {
    let result = client
        .inner
        .list_buckets()
        .send()
        .await
        .map_err(|e| S3Error::AwsError(format!("列出存储桶失败: {}", e)))?;

    // 转换为自定义BucketInfo结构
    let buckets = result
        .buckets()
        .iter()
        .map(|b: &Bucket| BucketInfo {
            name: b.name().unwrap_or_default().to_string(),
            created: b.creation_date().map(|d: &DateTime| d.to_string()),
        })
        .collect();

    Ok(buckets)
}

/// 检查存储桶是否存在
///
/// 使用HEAD操作检查存储桶是否存在。
///
/// # 参数
///
/// - `client`: S3客户端实例
/// - `bucket`: 存储桶名称
///
/// # 返回值
///
/// - `Ok(true)`: 桶存在
/// - `Ok(false)`: 桶不存在
pub async fn bucket_exists(
    client: &S3Client,
    bucket: &str,
) -> Result<bool, S3Error> {
    match client.inner.head_bucket().bucket(bucket).send().await {
        Ok(_) => Ok(true),
        Err(e) => {
            // 404表示桶不存在
            if e.as_service_error()
                .map(|se| se.is_not_found())
                .unwrap_or(false)
            {
                Ok(false)
            } else {
                Err(S3Error::AwsError(format!("检查存储桶失败: {}", e)))
            }
        }
    }
}

/// 设置存储桶CORS配置
///
/// 配置跨域资源共享规则,允许浏览器跨域访问。
///
/// # 参数
///
/// - `client`: S3客户端实例
/// - `bucket`: 存储桶名称
/// - `allowed_origins`: 允许的源列表(如["*"]或["https://example.com"])
/// - `allowed_methods`: 允许的HTTP方法(如["GET", "PUT"])
/// - `allowed_headers`: 允许的请求头
/// - `max_age_seconds`: 预检请求缓存时间(秒)
///
/// # 示例
///
/// ```rust,no_run
/// async fn setup_cors(client: &s3_service::S3Client) -> Result<(), s3_service::S3Error> {
///     s3_service::operations::put_bucket_cors(
///         client,
///         "my-bucket",
///         vec!["*".to_string()],
///         vec!["GET".to_string(), "PUT".to_string()],
///         vec!["*".to_string()],
///         Some(3600),
///     ).await
/// }
/// ```
pub async fn put_bucket_cors(
    client: &S3Client,
    bucket: &str,
    allowed_origins: Vec<String>,
    allowed_methods: Vec<String>,
    allowed_headers: Vec<String>,
    max_age_seconds: Option<i32>,
) -> Result<(), S3Error> {
    // 构建CORS规则
    let rule = aws_sdk_s3::types::CorsRule::builder()
        .set_allowed_origins(Some(allowed_origins))
        .set_allowed_methods(Some(allowed_methods))
        .set_allowed_headers(Some(allowed_headers))
        .max_age_seconds(max_age_seconds.unwrap_or(3600))
        .build()
        .map_err(|e| S3Error::AwsError(format!("构建CORS规则失败: {}", e)))?;

    // 构建CORS配置
    let cors_config = aws_sdk_s3::types::CorsConfiguration::builder()
        .cors_rules(rule)
        .build()
        .map_err(|e| S3Error::AwsError(format!("构建CORS配置失败: {}", e)))?;

    // 应用配置
    client
        .inner
        .put_bucket_cors()
        .bucket(bucket)
        .cors_configuration(cors_config)
        .send()
        .await
        .map_err(|e| S3Error::AwsError(format!("设置CORS失败: {}", e)))?;

    Ok(())
}

/// 设置存储桶访问策略
///
/// 设置存储桶的IAM访问策略,控制访问权限。
///
/// # 参数
///
/// - `client`: S3客户端实例
/// - `bucket`: 存储桶名称
/// - `policy`: JSON格式的策略字符串
pub async fn put_bucket_policy(
    client: &S3Client,
    bucket: &str,
    policy: &str,
) -> Result<(), S3Error> {
    client
        .inner
        .put_bucket_policy()
        .bucket(bucket)
        .policy(policy)
        .send()
        .await
        .map_err(|e| S3Error::AwsError(format!("设置桶策略失败: {}", e)))?;
    Ok(())
}

/// 获取存储桶访问策略
///
/// 获取存储桶当前的IAM访问策略。
///
/// # 参数
///
/// - `client`: S3客户端实例
/// - `bucket`: 存储桶名称
///
/// # 返回值
///
/// JSON格式的策略字符串
pub async fn get_bucket_policy(
    client: &S3Client,
    bucket: &str,
) -> Result<String, S3Error> {
    let result = client
        .inner
        .get_bucket_policy()
        .bucket(bucket)
        .send()
        .await
        .map_err(|e| S3Error::AwsError(format!("获取桶策略失败: {}", e)))?;

    Ok(result.policy().unwrap_or_default().to_string())
}

/// 存储桶信息结构体
///
/// 存储桶的基本信息,用于列举时返回。
#[derive(Debug, serde::Serialize)]
pub struct BucketInfo {
    /// 存储桶名称
    pub name: String,
    
    /// 创建时间
    pub created: Option<String>,
}
