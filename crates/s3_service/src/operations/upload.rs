//! 对象上传操作模块
//!
//! 提供对象上传和元数据设置功能。

use crate::client::S3Client;
use crate::error::S3Error;
use crate::storage::StorageInfo;

/// 上传对象到S3
///
/// 将数据上传到S3存储桶。
///
/// # 参数
///
/// - `client`: S3客户端实例
/// - `bucket`: 存储桶名称
/// - `key`: 对象键名
/// - `data`: 上传的数据
/// - `content_type`: 可选的MIME类型
///
/// # 返回值
///
/// 上传成功后返回存储信息
///
/// # 示例
///
/// ```rust,no_run
/// async fn upload_example(client: &s3_service::S3Client) -> Result<(), s3_service::S3Error> {
///     let data = b"Hello, S3!".to_vec();
///     let info = s3_service::operations::upload_object(
///         client,
///         "my-bucket",
///         "hello.txt",
///         data,
///         Some("text/plain")
///     ).await?;
///     
///     println!("Uploaded: {} ({} bytes)", info.key, info.size);
///     Ok(())
/// }
/// ```
pub async fn upload_object(
    client: &S3Client,
    bucket: &str,
    key: &str,
    data: Vec<u8>,
    content_type: Option<&str>,
) -> Result<StorageInfo, S3Error> {
    let size = data.len() as i64;

    // 构建上传请求
    let mut builder = client
        .inner
        .put_object()
        .bucket(bucket)
        .key(key);

    // 设置内容类型
    if let Some(ct) = content_type {
        builder = builder.content_type(ct);
    }

    // 执行上传
    let result = builder
        .body(data.into())
        .send()
        .await
        .map_err(|e| S3Error::AwsError(format!("上传对象失败: {}", e)))?;

    Ok(StorageInfo {
        bucket: Some(bucket.to_string()),
        key: key.to_string(),
        size,
        content_type: content_type.map(|s| s.to_string()),
        etag: result.e_tag().map(|s| s.to_string()),
        storage_type: crate::storage::StorageType::S3,
    })
}

/// 上传对象并设置自定义元数据
///
/// 除了上传数据外,还设置自定义元数据。
///
/// # 参数
///
/// - `client`: S3客户端实例
/// - `bucket`: 存储桶名称
/// - `key`: 对象键名
/// - `data`: 上传的数据
/// - `content_type`: MIME类型
/// - `metadata`: 自定义元数据键值对
///
/// # 元数据使用场景
///
/// - 存储文件描述信息
/// - 记录上传用户
/// - 保存自定义属性
///
/// # 示例
///
/// ```rust,no_run
/// use std::collections::HashMap;
///
/// async fn upload_with_meta(client: &s3_service::S3Client) -> Result<(), s3_service::S3Error> {
///     let mut metadata = HashMap::new();
///     metadata.insert("author".to_string(), "Alice".to_string());
///     metadata.insert("department".to_string(), "Engineering".to_string());
///     
///     let data = b"Document content".to_vec();
///     s3_service::operations::upload_object_with_metadata(
///         client,
///         "my-bucket",
///         "document.pdf",
///         data,
///         Some("application/pdf"),
///         metadata
///     ).await
/// }
/// ```
pub async fn upload_object_with_metadata(
    client: &S3Client,
    bucket: &str,
    key: &str,
    data: Vec<u8>,
    content_type: Option<&str>,
    metadata: std::collections::HashMap<String, String>,
) -> Result<StorageInfo, S3Error> {
    let size = data.len() as i64;

    // 构建上传请求
    let mut builder = client
        .inner
        .put_object()
        .bucket(bucket)
        .key(key);

    // 设置内容类型
    if let Some(ct) = content_type {
        builder = builder.content_type(ct);
    }

    // 设置自定义元数据
    for (k, v) in &metadata {
        builder = builder.metadata(k, v);
    }

    // 执行上传
    let result = builder
        .body(data.into())
        .send()
        .await
        .map_err(|e| S3Error::AwsError(format!("上传对象(带元数据)失败: {}", e)))?;

    Ok(StorageInfo {
        bucket: Some(bucket.to_string()),
        key: key.to_string(),
        size,
        content_type: content_type.map(|s| s.to_string()),
        etag: result.e_tag().map(|s| s.to_string()),
        storage_type: crate::storage::StorageType::S3,
    })
}