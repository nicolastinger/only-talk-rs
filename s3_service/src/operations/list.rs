//! 对象列举操作模块
//!
//! 提供存储桶中对象的列举和分页功能。

use aws_sdk_s3::types::{Object, ObjectStorageClass, CommonPrefix};
use aws_sdk_s3::primitives::DateTime;

use crate::client::S3Client;
use crate::error::S3Error;
use crate::storage::ObjectInfo;

/// 列举存储桶中的对象
///
/// 列出存储桶中的所有对象,支持前缀过滤和数量限制。
///
/// # 参数
///
/// - `client`: S3客户端实例
/// - `bucket`: 存储桶名称
/// - `prefix`: 可选的对象键前缀过滤
/// - `max_keys`: 返回的最大对象数量
///
/// # 返回值
///
/// 返回对象列表、公共前缀和分页信息
///
/// # 示例
///
/// ```rust,no_run
/// async fn list_example(client: &s3_service::S3Client) -> Result<(), s3_service::S3Error> {
///     let result = s3_service::operations::list_objects(
///         client,
///         "my-bucket",
///         Some("photos/"),
///         Some(100)
///     ).await?;
///     
///     for obj in result.objects {
///         println!("Object: {} ({} bytes)", obj.key, obj.size);
///     }
///     Ok(())
/// }
/// ```
pub async fn list_objects(
    client: &S3Client,
    bucket: &str,
    prefix: Option<&str>,
    max_keys: Option<i32>,
) -> Result<ListObjectsResult, S3Error> {
    // 构建列举请求
    let mut builder = client.inner.list_objects_v2().bucket(bucket);

    // 设置前缀过滤
    if let Some(p) = prefix {
        builder = builder.prefix(p);
    }

    // 设置返回数量
    if let Some(mk) = max_keys {
        builder = builder.max_keys(mk);
    }

    // 执行列举
    let result = builder
        .send()
        .await
        .map_err(|e| S3Error::AwsError(format!("列举对象失败: {}", e)))?;

    // 转换结果
    let objects = convert_objects(result.contents());
    let common_prefixes = extract_common_prefixes(result.common_prefixes());

    Ok(ListObjectsResult {
        objects,
        common_prefixes,
        is_truncated: result.is_truncated().unwrap_or(false),
        next_continuation_token: result
            .next_continuation_token()
            .map(|s: &str| s.to_string()),
    })
}

/// 分页列举对象
///
/// 支持分页token的列举操作,用于处理大量对象。
///
/// # 参数
///
/// - `client`: S3客户端实例
/// - `bucket`: 存储桶名称
/// - `prefix`: 可选的前缀过滤
/// - `max_keys`: 每页最大数量
/// - `continuation_token`: 分页token(上一页返回)
///
/// # 使用场景
///
/// 当对象数量超过1000或需要分页展示时使用
pub async fn list_objects_paginated(
    client: &S3Client,
    bucket: &str,
    prefix: Option<&str>,
    max_keys: Option<i32>,
    continuation_token: Option<&str>,
) -> Result<ListObjectsResult, S3Error> {
    // 构建列举请求
    let mut builder = client.inner.list_objects_v2().bucket(bucket);

    // 设置前缀
    if let Some(p) = prefix {
        builder = builder.prefix(p);
    }

    // 设置每页数量
    if let Some(mk) = max_keys {
        builder = builder.max_keys(mk);
    }

    // 设置分页token
    if let Some(token) = continuation_token {
        builder = builder.continuation_token(token);
    }

    // 执行列举
    let result = builder
        .send()
        .await
        .map_err(|e| S3Error::AwsError(format!("分页列举对象失败: {}", e)))?;

    // 转换结果
    let objects = convert_objects(result.contents());
    let common_prefixes = extract_common_prefixes(result.common_prefixes());

    Ok(ListObjectsResult {
        objects,
        common_prefixes,
        is_truncated: result.is_truncated().unwrap_or(false),
        next_continuation_token: result
            .next_continuation_token()
            .map(|s: &str| s.to_string()),
    })
}

/// 转换S3 Object列表为ObjectInfo列表
///
/// 内部函数,将AWS SDK的对象结构转换为自定义结构
pub(crate) fn convert_objects(contents: &[Object]) -> Vec<ObjectInfo> {
    contents
        .iter()
        .map(|obj| ObjectInfo {
            key: obj.key().unwrap_or_default().to_string(),
            size: obj.size().unwrap_or(0),
            last_modified: obj.last_modified().map(|t: &DateTime| t.to_string()),
            etag: obj.e_tag().map(|s: &str| s.to_string()),
            storage_class: obj.storage_class().map(|sc: &ObjectStorageClass| sc.as_str().to_string()),
        })
        .collect()
}

/// 提取公共前缀
///
/// 用于模拟目录结构,提取公共前缀(如目录名)
fn extract_common_prefixes(common_prefixes: &[CommonPrefix]) -> Vec<String> {
    common_prefixes
        .iter()
        .filter_map(|p| p.prefix().map(|s: &str| s.to_string()))
        .collect()
}

/// 列举对象结果
///
/// 包含对象列表、公共前缀和分页信息
#[derive(Debug, serde::Serialize)]
pub struct ListObjectsResult {
    /// 对象列表
    pub objects: Vec<ObjectInfo>,
    
    /// 公共前缀(用于模拟目录结构)
    pub common_prefixes: Vec<String>,
    
    /// 是否还有更多结果
    pub is_truncated: bool,
    
    /// 下一页的continuation token
    /// 用于获取下一页数据
    pub next_continuation_token: Option<String>,
}