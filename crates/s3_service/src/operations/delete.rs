//! 对象删除操作模块
//!
//! 提供单个和批量对象删除功能。

use aws_sdk_s3::types::{DeletedObject, Error as S3DeleteError};

use crate::client::S3Client;
use crate::error::S3Error as AppS3Error;

/// 删除单个对象
///
/// 从桶中删除指定对象。
///
/// # 参数
///
/// - `client`: S3 客户端实例
/// - `bucket`: 桶名称
/// - `key`: 对象键名
///
/// # 说明
///
/// - 删除不存在的对象不会报错
/// - 删除操作不可撤销
pub async fn delete_object(client: &S3Client, bucket: &str, key: &str) -> Result<(), AppS3Error> {
    client
        .inner
        .delete_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .map_err(|e| AppS3Error::AwsError(format!("Failed to delete object: {}", e)))?;
    Ok(())
}

/// 批量删除对象
///
/// 一次性删除多个对象，比逐个删除更高效。
///
/// # 参数
///
/// - `client`: S3 客户端实例
/// - `bucket`: 桶名称
/// - `keys`: 对象键名数组
///
/// # 返回值
///
/// 返回删除结果，包括成功和失败列表
///
/// # 性能
///
/// 单次请求最多可删除 1000 个对象，
/// 超过 1000 个需要分批处理
pub async fn delete_objects(
    client: &S3Client,
    bucket: &str,
    keys: &[&str],
) -> Result<DeleteBatchResult, AppS3Error> {
    // 空列表快速返回
    if keys.is_empty() {
        return Ok(DeleteBatchResult { deleted: Vec::new(), failed: Vec::new() });
    }

    // 构建删除请求的对象列表
    let objects: Vec<aws_sdk_s3::types::ObjectIdentifier> = keys
        .iter()
        .map(|k| {
            aws_sdk_s3::types::ObjectIdentifier::builder()
                .key(*k)
                .build()
                .expect("builder has required key field set")
        })
        .collect();

    // 构建删除请求
    let delete = aws_sdk_s3::types::Delete::builder()
        .set_objects(Some(objects))
        .quiet(false) // 返回删除结果
        .build()
        .expect("delete builder has all fields valid");

    // 执行批量删除
    let result = client
        .inner
        .delete_objects()
        .bucket(bucket)
        .delete(delete)
        .send()
        .await
        .map_err(|e| AppS3Error::AwsError(format!("Failed to batch delete objects: {}", e)))?;

    // 提取成功删除的对象
    let deleted = result
        .deleted()
        .iter()
        .filter_map(|obj: &DeletedObject| obj.key().map(|s: &str| s.to_string()))
        .collect::<Vec<String>>();

    // 提取失败的对象
    let failed = result
        .errors()
        .iter()
        .map(|e: &S3DeleteError| DeleteError {
            key: e.key().unwrap_or_default().to_string(),
            code: e.code().unwrap_or_default().to_string(),
            message: e.message().unwrap_or_default().to_string(),
        })
        .collect::<Vec<DeleteError>>();

    Ok(DeleteBatchResult { deleted, failed })
}

/// 批量删除结果
///
/// 包含成功删除和失败的对象信息
#[derive(Debug, serde::Serialize)]
pub struct DeleteBatchResult {
    /// 成功删除的对象键
    pub deleted: Vec<String>,

    /// 失败对象列表
    pub failed: Vec<DeleteError>,
}

/// 删除错误信息
///
/// 对象删除失败时的详细信息
#[derive(Debug, serde::Serialize)]
pub struct DeleteError {
    /// 对象键名
    pub key: String,

    /// 错误码
    pub code: String,

    /// 错误消息
    pub message: String,
}
