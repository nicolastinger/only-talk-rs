//! 对象下载操作模块
//!
//! 提供对象下载和 Range 范围下载功能。

use crate::client::S3Client;
use crate::error::S3Error;

/// 下载对象
///
/// 将对象的完整内容下载到内存中。
///
/// # 参数
///
/// - `client`: S3 客户端实例
/// - `bucket`: 存储桶名称
/// - `key`: 对象键名
///
/// # 返回值
///
/// 对象的二进制数据
///
/// # 说明
///
/// 大文件下载会消耗大量内存，
/// 可考虑使用 download_object_range 进行分段下载
pub async fn download_object(
    client: &S3Client,
    bucket: &str,
    key: &str,
) -> Result<Vec<u8>, S3Error> {
    let result = client.inner.get_object().bucket(bucket).key(key).send().await.map_err(|e| {
        // 处理对象不存在错误
        if e.as_service_error().map(|se| se.is_no_such_key()).unwrap_or(false) {
            S3Error::ObjectNotFound(key.to_string())
        } else {
            S3Error::AwsError(format!("Failed to download object: {}", e))
        }
    })?;

    // 收集响应流数据
    let data = result
        .body
        .collect()
        .await
        .map_err(|e| S3Error::AwsError(format!("Failed to read object data: {}", e)))?;

    Ok(data.into_bytes().to_vec())
}

/// Range 范围下载对象
///
/// 下载对象指定的字节范围，支持断点续传。
///
/// # 参数
///
/// - `client`: S3 客户端实例
/// - `bucket`: 存储桶名称
/// - `key`: 对象键名
/// - `start`: 起始字节位置（含）
/// - `end`: 结束字节位置（含）
///
/// # 返回值
///
/// 指定范围内的数据
///
/// # 使用场景
///
/// - 断点续传：记录已下载位置，继续下载剩余部分
/// - 分块下载：将大文件拆分为多个块分别下载
/// - 预览：仅下载文件头信息
///
/// # HTTP Range 格式
///
/// 使用 "bytes=start-end" 格式，
/// 例如 bytes=0-1023 表示下载前 1024 字节
pub async fn download_object_range(
    client: &S3Client,
    bucket: &str,
    key: &str,
    start: i64,
    end: i64,
) -> Result<Vec<u8>, S3Error> {
    // 构建 Range 请求头
    let range = format!("bytes={}-{}", start, end);
    let result = client
        .inner
        .get_object()
        .bucket(bucket)
        .key(key)
        .range(range)
        .send()
        .await
        .map_err(|e| S3Error::AwsError(format!("Failed to Range download object: {}", e)))?;

    // 收集响应流数据
    let data = result
        .body
        .collect()
        .await
        .map_err(|e| S3Error::AwsError(format!("Failed to read object data: {}", e)))?;

    Ok(data.into_bytes().to_vec())
}

/// 将对象下载到本地文件
///
/// 下载对象并直接写入本地文件。
///
/// # 参数
///
/// - `client`: S3 客户端实例
/// - `bucket`: 存储桶名称
/// - `key`: 对象键名
/// - `file_path`: 本地文件保存路径
///
/// # 说明
///
/// 自动创建所需的目录结构
pub async fn download_object_to_file(
    client: &S3Client,
    bucket: &str,
    key: &str,
    file_path: &str,
) -> Result<(), S3Error> {
    // 下载对象数据
    let data = download_object(client, bucket, key).await?;

    // 确保父目录存在
    if let Some(parent) = std::path::Path::new(file_path).parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    // 写入文件
    tokio::fs::write(file_path, &data).await?;
    Ok(())
}
