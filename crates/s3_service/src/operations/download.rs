//! 对象下载操作模块
//!
//! 提供对象下载和Range下载功能。

use crate::client::S3Client;
use crate::error::S3Error;

/// 下载对象
///
/// 下载对象的完整内容到内存。
///
/// # 参数
///
/// - `client`: S3客户端实例
/// - `bucket`: 存储桶名称
/// - `key`: 对象键名
///
/// # 返回值
///
/// 对象的二进制数据
///
/// # 注意事项
///
/// 大文件下载会占用大量内存,
/// 建议使用download_object_range分批下载
pub async fn download_object(
    client: &S3Client,
    bucket: &str,
    key: &str,
) -> Result<Vec<u8>, S3Error> {
    let result = client
        .inner
        .get_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .map_err(|e| {
            // 处理对象不存在的错误
            if e.as_service_error().map(|se| se.is_no_such_key()).unwrap_or(false) {
                S3Error::ObjectNotFound(key.to_string())
            } else {
                S3Error::AwsError(format!("下载对象失败: {}", e))
            }
        })?;

    // 收集响应流数据
    let data = result
        .body
        .collect()
        .await
        .map_err(|e| S3Error::AwsError(format!("读取对象数据失败: {}", e)))?;

    Ok(data.into_bytes().to_vec())
}

/// Range下载对象
///
/// 下载对象的指定字节范围,支持断点续传。
///
/// # 参数
///
/// - `client`: S3客户端实例
/// - `bucket`: 存储桶名称
/// - `key`: 对象键名
/// - `start`: 起始字节位置(包含)
/// - `end`: 结束字节位置(包含)
///
/// # 返回值
///
/// 指定范围的数据
///
/// # 使用场景
///
/// - 断点续传: 记录已下载位置,继续下载剩余部分
/// - 分块下载: 大文件分多块下载
/// - 预览: 只下载文件头部信息
///
/// # HTTP Range格式
///
/// 使用"bytes=start-end"格式,
/// 如bytes=0-1023表示下载前1024字节
pub async fn download_object_range(
    client: &S3Client,
    bucket: &str,
    key: &str,
    start: i64,
    end: i64,
) -> Result<Vec<u8>, S3Error> {
    // 构建Range请求头
    let range = format!("bytes={}-{}", start, end);
    let result = client
        .inner
        .get_object()
        .bucket(bucket)
        .key(key)
        .range(range)
        .send()
        .await
        .map_err(|e| S3Error::AwsError(format!("Range下载对象失败: {}", e)))?;

    // 收集响应流数据
    let data = result
        .body
        .collect()
        .await
        .map_err(|e| S3Error::AwsError(format!("读取对象数据失败: {}", e)))?;

    Ok(data.into_bytes().to_vec())
}

/// 下载对象到本地文件
///
/// 下载对象并直接写入本地文件。
///
/// # 参数
///
/// - `client`: S3客户端实例
/// - `bucket`: 存储桶名称
/// - `key`: 对象键名
/// - `file_path`: 本地文件保存路径
///
/// # 注意事项
///
/// 会自动创建文件所需的目录结构
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