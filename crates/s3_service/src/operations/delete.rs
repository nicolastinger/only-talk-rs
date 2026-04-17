//! 对象删除操作模块
//!
//! 提供单个和批量删除对象的功能。

use aws_sdk_s3::types::{Error as S3DeleteError, DeletedObject};

use crate::client::S3Client;
use crate::error::S3Error as AppS3Error;

/// 删除单个对象
///
/// 从存储桶中删除指定对象。
///
/// # 参数
///
/// - `client`: S3客户端实例
/// - `bucket`: 存储桶名称
/// - `key`: 对象键名
///
/// # 注意事项
///
/// - 删除不存在的对象不会报错
/// - 删除操作不可逆
pub async fn delete_object(
    client: &S3Client,
    bucket: &str,
    key: &str,
) -> Result<(), AppS3Error> {
    client
        .inner
        .delete_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .map_err(|e| AppS3Error::AwsError(format!("删除对象失败: {}", e)))?;
    Ok(())
}

/// 批量删除对象
///
/// 一次性删除多个对象,比单个删除更高效。
///
/// # 参数
///
/// - `client`: S3客户端实例
/// - `bucket`: 存储桶名称
/// - `keys`: 对象键名数组
///
/// # 返回值
///
/// 返回删除结果,包含成功和失败的列表
///
/// # 性能
///
/// 单次请求最多可删除1000个对象,
/// 超过1000个需分批处理
pub async fn delete_objects(
    client: &S3Client,
    bucket: &str,
    keys: &[&str],
) -> Result<DeleteBatchResult, AppS3Error> {
    // 空列表快速返回
    if keys.is_empty() {
        return Ok(DeleteBatchResult {
            deleted: Vec::new(),
            failed: Vec::new(),
        });
    }

    // 构建删除请求的对象列表
    let objects: Vec<aws_sdk_s3::types::ObjectIdentifier> = keys
        .iter()
        .map(|k| {
            aws_sdk_s3::types::ObjectIdentifier::builder()
                .key(*k)
                .build()
                .unwrap()
        })
        .collect();

    // 构建删除请求
    let delete = aws_sdk_s3::types::Delete::builder()
        .set_objects(Some(objects))
        .quiet(false)  // 返回删除结果
        .build()
        .unwrap();

    // 执行批量删除
    let result = client
        .inner
        .delete_objects()
        .bucket(bucket)
        .delete(delete)
        .send()
        .await
        .map_err(|e| AppS3Error::AwsError(format!("批量删除对象失败: {}", e)))?;

    // 提取成功删除的对象
    let deleted = result
        .deleted()
        .iter()
        .filter_map(|obj: &DeletedObject| obj.key().map(|s: &str| s.to_string()))
        .collect::<Vec<String>>();

    // 提取删除失败的对象
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
/// 包含成功删除和删除失败的对象信息
#[derive(Debug, serde::Serialize)]
pub struct DeleteBatchResult {
    /// 成功删除的对象key列表
    pub deleted: Vec<String>,
    
    /// 删除失败的对象列表
    pub failed: Vec<DeleteError>,
}

/// 删除错误信息
///
/// 单个对象删除失败的详细信息
#[derive(Debug, serde::Serialize)]
pub struct DeleteError {
    /// 对象键名
    pub key: String,
    
    /// 错误代码
    pub code: String,
    
    /// 错误消息
    pub message: String,
}