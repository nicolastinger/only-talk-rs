//! 分片上传操作模块
//!
//! 提供大文件的分片上传功能。

use aws_sdk_s3::types::MultipartUpload;
use aws_sdk_s3::primitives::DateTime;

use crate::client::S3Client;
use crate::error::S3Error;
use crate::storage::StorageInfo;

/// 分片上传大文件
///
/// 将大文件分成多个部分并行上传,提高上传效率。
///
/// # 参数
///
/// - `client`: S3客户端实例
/// - `bucket`: 存储桶名称
/// - `key`: 对象键名
/// - `data`: 文件数据
/// - `size`: 文件大小(字节)
/// - `content_type`: MIME类型
///
/// # 工作流程
///
/// 1. 调用CreateMultipartUpload初始化上传
/// 2. 将文件分片并逐个上传(UploadPart)
/// 3. 所有分片上传完成后调用CompleteMultipartUpload
///
/// # 分片大小
///
/// 使用配置中的multipart_chunk_size,默认5MB
///
/// # 错误处理
///
/// 上传失败时应调用abort_multipart_upload清理
pub async fn multipart_upload(
    client: &S3Client,
    bucket: &str,
    key: &str,
    data: Vec<u8>,
    size: i64,
    content_type: Option<&str>,
) -> Result<StorageInfo, S3Error> {
    let chunk_size = client.config.multipart_chunk_size as usize;

    // 步骤1: 初始化分片上传
    let mut create_builder = client
        .inner
        .create_multipart_upload()
        .bucket(bucket)
        .key(key);

    // 设置内容类型
    if let Some(ct) = content_type {
        create_builder = create_builder.content_type(ct);
    }

    // 获取upload_id
    let create_result = create_builder
        .send()
        .await
        .map_err(|e| S3Error::MultipartError(format!("初始化分片上传失败: {}", e)))?;

    let upload_id = create_result
        .upload_id()
        .ok_or_else(|| S3Error::MultipartError("未获取到upload_id".to_string()))?
        .to_string();

    // 步骤2: 上传各个分片
    let mut completed_parts = Vec::new();
    let mut part_number: i32 = 0;

    for chunk in data.chunks(chunk_size) {
        part_number += 1;
        
        // 上传单个分片
        let upload_part_result = client
            .inner
            .upload_part()
            .bucket(bucket)
            .key(key)
            .upload_id(&upload_id)
            .part_number(part_number)
            .body(chunk.to_vec().into())
            .send()
            .await
            .map_err(|e| {
                S3Error::MultipartError(format!("上传分片 #{} 失败: {}", part_number, e))
            })?;

        // 记录已上传的分片信息
        completed_parts.push(
            aws_sdk_s3::types::CompletedPart::builder()
                .part_number(part_number)
                .e_tag(
                    upload_part_result
                        .e_tag()
                        .unwrap_or_default()
                        .to_string(),
                )
                .build(),
        );
    }

    // 步骤3: 完成分片上传
    let completed_multipart_upload = aws_sdk_s3::types::CompletedMultipartUpload::builder()
        .set_parts(Some(completed_parts))
        .build();

    client
        .inner
        .complete_multipart_upload()
        .bucket(bucket)
        .key(key)
        .upload_id(&upload_id)
        .multipart_upload(completed_multipart_upload)
        .send()
        .await
        .map_err(|e| S3Error::MultipartError(format!("完成分片上传失败: {}", e)))?;

    Ok(StorageInfo {
        bucket: Some(bucket.to_string()),
        key: key.to_string(),
        size,
        content_type: content_type.map(|s| s.to_string()),
        etag: None,
        storage_type: crate::storage::StorageType::S3,
    })
}

/// 中止分片上传
///
/// 取消正在进行的分片上传,清理已上传的分片。
///
/// # 参数
///
/// - `client`: S3客户端实例
/// - `bucket`: 存储桶名称
/// - `key`: 对象键名
/// - `upload_id`: 分片上传ID
///
/// # 使用场景
///
/// 上传失败或中断时,应调用此函数清理资源
pub async fn abort_multipart_upload(
    client: &S3Client,
    bucket: &str,
    key: &str,
    upload_id: &str,
) -> Result<(), S3Error> {
    client
        .inner
        .abort_multipart_upload()
        .bucket(bucket)
        .key(key)
        .upload_id(upload_id)
        .send()
        .await
        .map_err(|e| S3Error::MultipartError(format!("中止分片上传失败: {}", e)))?;
    Ok(())
}

/// 列出未完成的分片上传
///
/// 列出存储桶中所有未完成的分片上传任务。
///
/// # 参数
///
/// - `client`: S3客户端实例
/// - `bucket`: 存储桶名称
/// - `prefix`: 可选的对象键前缀过滤
///
/// # 返回值
///
/// 未完成的分片上传信息列表
///
/// # 使用场景
///
/// - 监控上传进度
/// - 清理长时间未完成的上传
pub async fn list_multipart_uploads(
    client: &S3Client,
    bucket: &str,
    prefix: Option<&str>,
) -> Result<Vec<MultipartUploadInfo>, S3Error> {
    // 构建列举请求
    let mut builder = client
        .inner
        .list_multipart_uploads()
        .bucket(bucket);

    if let Some(p) = prefix {
        builder = builder.prefix(p);
    }

    // 执行列举
    let result = builder
        .send()
        .await
        .map_err(|e| S3Error::MultipartError(format!("列出分片上传失败: {}", e)))?;

    // 转换结果
    let uploads = result
        .uploads()
        .iter()
        .map(|u: &MultipartUpload| MultipartUploadInfo {
            key: u.key().unwrap_or_default().to_string(),
            upload_id: u.upload_id().unwrap_or_default().to_string(),
            initiated: u.initiated().map(|t: &DateTime| t.to_string()),
        })
        .collect();

    Ok(uploads)
}

/// 分片上传信息
///
/// 记录未完成的分片上传任务信息
#[derive(Debug)]
pub struct MultipartUploadInfo {
    /// 对象键名
    pub key: String,
    
    /// 分片上传ID
    pub upload_id: String,
    
    /// 初始化时间
    pub initiated: Option<String>,
}