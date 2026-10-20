//! 预签名 URL 操作模块
//!
//! 提供生成临时访问 URL 的功能。

use std::time::Duration;

use crate::client::S3Client;
use crate::error::S3Error;
use crate::storage::PresignedMethod;

/// 生成预签名 URL
///
/// 生成一个临时的、带签名的 URL，允许直接访问对象。
///
/// # 参数
///
/// - `client`: S3 客户端实例
/// - `bucket`: 存储桶名称
/// - `key`: 对象键名
/// - `expires`: URL 过期时间
/// - `method`: HTTP 方法（GET 用于下载，PUT 用于上传）
///
/// # 返回值
///
/// 签名后的临时 URL 字符串
///
/// # 安全性
///
/// - URL 包含签名，无法伪造
/// - 超过过期时间后自动失效
/// - 可以限制操作类型（GET/PUT）
///
/// # 使用场景
///
/// - 浏览器直接下载/上传
/// - 分享临时访问链接
/// - 减轻服务器负载（直传）
///
/// # 示例
///
/// ```rust,no_run
/// use std::time::Duration;
/// use s3_service::storage::PresignedMethod;
///
/// async fn example(client: &s3_service::S3Client) -> Result<String, s3_service::S3Error> {
///     // Generate download URL, expires after 1 hour
///     let url = s3_service::operations::generate_presigned_url(
///         client,
///         "my-bucket",
///         "file.pdf",
///         Duration::from_secs(3600),
///         PresignedMethod::Get
///     ).await?;
///     Ok(url)
/// }
/// ```
pub async fn generate_presigned_url(
    client: &S3Client,
    bucket: &str,
    key: &str,
    expires: Duration,
    method: PresignedMethod,
) -> Result<String, S3Error> {
    let expires_secs = expires.as_secs() as u32;

    match method {
        PresignedMethod::Get => {
            // 生成下载预签名 URL
            let builder = client.inner.get_object().bucket(bucket).key(key);

            let presigned_request = builder
                .presigned(
                    aws_sdk_s3::presigning::PresigningConfig::expires_in(Duration::from_secs(
                        expires_secs as u64,
                    ))
                    .map_err(|e| {
                        S3Error::PresignError(format!("Failed to configure presigned URL: {}", e))
                    })?,
                )
                .await
                .map_err(|e| {
                    S3Error::PresignError(format!(
                        "Failed to generate download presigned URL: {}",
                        e
                    ))
                })?;

            Ok(presigned_request.uri().to_string())
        }
        PresignedMethod::Put => {
            // 生成上传预签名 URL
            let builder = client.inner.put_object().bucket(bucket).key(key);

            let presigned_request = builder
                .presigned(
                    aws_sdk_s3::presigning::PresigningConfig::expires_in(Duration::from_secs(
                        expires_secs as u64,
                    ))
                    .map_err(|e| {
                        S3Error::PresignError(format!("Failed to configure presigned URL: {}", e))
                    })?,
                )
                .await
                .map_err(|e| {
                    S3Error::PresignError(format!("Failed to generate upload presigned URL: {}", e))
                })?;

            Ok(presigned_request.uri().to_string())
        }
    }
}

/// 生成下载预签名 URL
///
/// 便捷函数，专门用于生成下载 URL。
///
/// # 参数
///
/// - `client`: S3 客户端实例
/// - `bucket`: 存储桶名称
/// - `key`: 对象键名
/// - `expires`: 过期时间
pub async fn generate_download_presigned_url(
    client: &S3Client,
    bucket: &str,
    key: &str,
    expires: Duration,
) -> Result<String, S3Error> {
    generate_presigned_url(client, bucket, key, expires, PresignedMethod::Get).await
}

/// 生成上传预签名 URL
///
/// 便捷函数，专门用于生成上传 URL。
///
/// # 参数
///
/// - `client`: S3 客户端实例
/// - `bucket`: 存储桶名称
/// - `key`: 对象键名
/// - `expires`: 过期时间
pub async fn generate_upload_presigned_url(
    client: &S3Client,
    bucket: &str,
    key: &str,
    expires: Duration,
) -> Result<String, S3Error> {
    generate_presigned_url(client, bucket, key, expires, PresignedMethod::Put).await
}

/// 使用默认过期时间生成下载预签名 URL
///
/// 使用配置中的默认过期时间。
///
/// # 参数
///
/// - `client`: S3 客户端实例
/// - `bucket`: 存储桶名称
/// - `key`: 对象键名
pub async fn generate_download_url_default(
    client: &S3Client,
    bucket: &str,
    key: &str,
) -> Result<String, S3Error> {
    // 使用配置的默认过期时间
    let expires = Duration::from_secs(client.config.presign_expire_seconds);
    generate_download_presigned_url(client, bucket, key, expires).await
}

/// 使用默认过期时间生成上传预签名 URL
///
/// 使用配置中的默认过期时间。
///
/// # 参数
///
/// - `client`: S3 客户端实例
/// - `bucket`: 存储桶名称
/// - `key`: 对象键名
pub async fn generate_upload_url_default(
    client: &S3Client,
    bucket: &str,
    key: &str,
) -> Result<String, S3Error> {
    // 使用配置的默认过期时间
    let expires = Duration::from_secs(client.config.presign_expire_seconds);
    generate_upload_presigned_url(client, bucket, key, expires).await
}
