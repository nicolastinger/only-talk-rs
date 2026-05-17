//! 对象元数据操作模块
//!
//! 提供对象元数据的获取、更新和标签管理功能。

use aws_sdk_s3::primitives::DateTime;
use aws_sdk_s3::types::Tag;

use crate::client::S3Client;
use crate::error::S3Error;
use crate::storage::ObjectMetadata;

/// 获取对象元数据
///
/// 使用HEAD操作获取对象元数据,不下载对象内容。
///
/// # 参数
///
/// - `client`: S3客户端实例
/// - `bucket`: 存储桶名称
/// - `key`: 对象键名
///
/// # 返回值
///
/// 对象的完整元数据信息
///
/// # 性能
///
/// HEAD操作不传输对象内容,速度快,开销小
pub async fn head_object(
    client: &S3Client,
    bucket: &str,
    key: &str,
) -> Result<ObjectMetadata, S3Error> {
    let result = client
        .inner
        .head_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .map_err(|e| {
            // 处理对象不存在的错误
            if e.as_service_error()
                .map(|se| se.is_not_found())
                .unwrap_or(false)
            {
                S3Error::ObjectNotFound(key.to_string())
            } else {
                S3Error::AwsError(format!("获取对象元数据失败: {}", e))
            }
        })?;

    // 提取自定义元数据
    let mut metadata = std::collections::HashMap::new();
    if let Some(meta) = result.metadata() {
        for (k, v) in meta.iter() {
            metadata.insert(k.clone(), v.clone());
        }
    }

    Ok(ObjectMetadata {
        key: key.to_string(),
        size: result.content_length().unwrap_or(0),
        content_type: result.content_type().map(|s| s.to_string()),
        last_modified: result.last_modified().map(|t: &DateTime| t.to_string()),
        etag: result.e_tag().map(|s| s.to_string()),
        metadata,
    })
}

/// 检查对象是否存在
///
/// 快速检查对象是否存在,不获取完整元数据。
///
/// # 参数
///
/// - `client`: S3客户端实例
/// - `bucket`: 存储桶名称
/// - `key`: 对象键名
///
/// # 返回值
///
/// - `Ok(true)`: 对象存在
/// - `Ok(false)`: 对象不存在
pub async fn object_exists(
    client: &S3Client,
    bucket: &str,
    key: &str,
) -> Result<bool, S3Error> {
    match head_object(client, bucket, key).await {
        Ok(_) => Ok(true),
        Err(S3Error::ObjectNotFound(_)) => Ok(false),
        Err(e) => Err(e),
    }
}

/// 更新对象元数据
///
/// 通过复制自身来更新对象的元数据。
///
/// # 参数
///
/// - `client`: S3客户端实例
/// - `bucket`: 存储桶名称
/// - `key`: 对象键名
/// - `content_type`: 可选的新内容类型
/// - `metadata`: 新的自定义元数据
///
/// # 实现原理
///
/// S3不支持直接修改元数据,
/// 通过COPY操作并替换元数据实现
pub async fn update_object_metadata(
    client: &S3Client,
    bucket: &str,
    key: &str,
    content_type: Option<&str>,
    metadata: std::collections::HashMap<String, String>,
) -> Result<(), S3Error> {
    // 复制源: 同一对象
    let copy_source = format!("{}/{}", bucket, key);

    // 构建复制请求
    let mut builder = client
        .inner
        .copy_object()
        .bucket(bucket)
        .key(key)
        .copy_source(copy_source)
        .metadata_directive(aws_sdk_s3::types::MetadataDirective::Replace);

    // 设置内容类型
    if let Some(ct) = content_type {
        builder = builder.content_type(ct);
    }

    // 设置自定义元数据
    for (k, v) in &metadata {
        builder = builder.metadata(k, v);
    }

    // 执行更新
    builder
        .send()
        .await
        .map_err(|e| S3Error::AwsError(format!("更新对象元数据失败: {}", e)))?;

    Ok(())
}

/// 设置对象标签
///
/// 为对象添加键值对标签,用于分类和管理。
///
/// # 参数
///
/// - `client`: S3客户端实例
/// - `bucket`: 存储桶名称
/// - `key`: 对象键名
/// - `tags`: 标签键值对集合
///
/// # 标签限制
///
/// - 每个对象最多50个标签
/// - 标签键最大128字节
/// - 标签值最大256字节
pub async fn put_object_tagging(
    client: &S3Client,
    bucket: &str,
    key: &str,
    tags: std::collections::HashMap<String, String>,
) -> Result<(), S3Error> {
    // 构建标签列表
    let tag_set: Vec<Tag> = tags
        .into_iter()
        .map(|(k, v)| {
            Tag::builder()
                .key(k)
                .value(v)
                .build()
                .unwrap()
        })
        .collect();

    // 构建标签配置
    let tagging = aws_sdk_s3::types::Tagging::builder()
        .set_tag_set(Some(tag_set))
        .build()
        .map_err(|e| S3Error::AwsError(format!("构建标签失败: {}", e)))?;

    // 应用标签
    client
        .inner
        .put_object_tagging()
        .bucket(bucket)
        .key(key)
        .tagging(tagging)
        .send()
        .await
        .map_err(|e| S3Error::AwsError(format!("设置对象标签失败: {}", e)))?;

    Ok(())
}

/// 获取对象标签
///
/// 获取对象的所有标签。
///
/// # 参数
///
/// - `client`: S3客户端实例
/// - `bucket`: 存储桶名称
/// - `key`: 对象键名
///
/// # 返回值
///
/// 标签键值对集合
pub async fn get_object_tagging(
    client: &S3Client,
    bucket: &str,
    key: &str,
) -> Result<std::collections::HashMap<String, String>, S3Error> {
    let result = client
        .inner
        .get_object_tagging()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .map_err(|e| S3Error::AwsError(format!("获取对象标签失败: {}", e)))?;

    // 转换为HashMap
    let tags = result
        .tag_set()
        .iter()
        .map(|t: &Tag| {
            (
                t.key().to_string(),
                t.value().to_string(),
            )
        })
        .collect();

    Ok(tags)
}