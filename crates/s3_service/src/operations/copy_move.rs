//! 对象复制和移动操作模块
//!
//! 提供对象复制和移动功能。

use crate::client::S3Client;
use crate::error::S3Error;

/// 复制对象
///
/// 在不同桶之间复制对象。
///
/// # 参数
///
/// - `client`: S3 客户端实例
/// - `source_bucket`: 源桶
/// - `source_key`: 源对象键名
/// - `dest_bucket`: 目标桶
/// - `dest_key`: 目标对象键名
///
/// # 示例
///
/// ```rust,no_run
/// async fn copy_example(client: &s3_service::S3Client) -> Result<(), s3_service::S3Error> {
///     s3_service::operations::copy_object(
///         client,
///         "source-bucket",
///         "file.txt",
///         "dest-bucket",
///         "file-copy.txt"
///     ).await
/// }
/// ```
pub async fn copy_object(
    client: &S3Client,
    source_bucket: &str,
    source_key: &str,
    dest_bucket: &str,
    dest_key: &str,
) -> Result<(), S3Error> {
    // 构建复制源路径：桶/键 格式
    let copy_source = format!("{}/{}", source_bucket, source_key);
    client
        .inner
        .copy_object()
        .bucket(dest_bucket)
        .key(dest_key)
        .copy_source(copy_source)
        .send()
        .await
        .map_err(|e| S3Error::AwsError(format!("Failed to copy object: {}", e)))?;
    Ok(())
}

/// 移动对象
///
/// 将对象从一个位置移动到另一个位置。
/// 实现方式：先复制，再删除源对象。
///
/// # 参数
///
/// - `client`: S3 客户端实例
/// - `source_bucket`: 源桶
/// - `source_key`: 源对象键名
/// - `dest_bucket`: 目标桶
/// - `dest_key`: 目标对象键名
///
/// # 说明
///
/// 移动操作是原子的，复制失败时不会删除源对象
pub async fn move_object(
    client: &S3Client,
    source_bucket: &str,
    source_key: &str,
    dest_bucket: &str,
    dest_key: &str,
) -> Result<(), S3Error> {
    // 先复制对象
    copy_object(client, source_bucket, source_key, dest_bucket, dest_key).await?;

    // 仅当源和目标不同时才删除源对象
    if source_bucket != dest_bucket || source_key != dest_key {
        client.inner.delete_object().bucket(source_bucket).key(source_key).send().await.map_err(
            |e| S3Error::AwsError(format!("Failed to delete source object during move: {}", e)),
        )?;
    }

    Ok(())
}

/// 在同一桶内复制对象
///
/// 简化版复制操作，源和目标在同一桶中。
///
/// # 参数
///
/// - `client`: S3 客户端实例
/// - `bucket`: 桶名称
/// - `source_key`: 源对象键名
/// - `dest_key`: 目标对象键名
pub async fn copy_object_same_bucket(
    client: &S3Client,
    bucket: &str,
    source_key: &str,
    dest_key: &str,
) -> Result<(), S3Error> {
    copy_object(client, bucket, source_key, bucket, dest_key).await
}

/// 在同一桶内移动对象
///
/// 简化版移动操作，源和目标在同一桶中。
///
/// # 参数
///
/// - `client`: S3 客户端实例
/// - `bucket`: 桶名称
/// - `source_key`: 源对象键名
/// - `dest_key`: 目标对象键名
pub async fn move_object_same_bucket(
    client: &S3Client,
    bucket: &str,
    source_key: &str,
    dest_key: &str,
) -> Result<(), S3Error> {
    move_object(client, bucket, source_key, bucket, dest_key).await
}
