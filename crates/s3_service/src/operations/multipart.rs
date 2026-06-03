//! Multipart upload operations module
//!
//! Provides multipart upload functionality for large files.

use aws_sdk_s3::types::MultipartUpload;
use aws_sdk_s3::primitives::DateTime;

use crate::client::S3Client;
use crate::error::S3Error;
use crate::storage::StorageInfo;

/// Multipart upload large file
///
/// Split large files into multiple parts for parallel upload, improving upload efficiency.
///
/// # Parameters
///
/// - `client`: S3 client instance
/// - `bucket`: Storage bucket name
/// - `key`: Object key name
/// - `data`: File data
/// - `size`: File size (bytes)
/// - `content_type`: MIME type
///
/// # Workflow
///
/// 1. Call CreateMultipartUpload to initialize upload
/// 2. Split file into parts and upload each individually (UploadPart)
/// 3. After all parts are uploaded, call CompleteMultipartUpload
///
/// # Chunk Size
///
/// Uses multipart_chunk_size from config, default 5MB
///
/// # Error Handling
///
/// On upload failure, should call abort_multipart_upload to clean up
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
        .map_err(|e| S3Error::MultipartError(format!("Failed to initialize multipart upload: {}", e)))?;

    let upload_id = create_result
        .upload_id()
        .ok_or_else(|| S3Error::MultipartError("Failed to get upload_id".to_string()))?
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
                S3Error::MultipartError(format!("Failed to upload part #{}: {}", part_number, e))
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
        .map_err(|e| S3Error::MultipartError(format!("Failed to complete multipart upload: {}", e)))?;

    Ok(StorageInfo {
        bucket: Some(bucket.to_string()),
        key: key.to_string(),
        size,
        content_type: content_type.map(|s| s.to_string()),
        etag: None,
        storage_type: crate::storage::StorageType::S3,
    })
}

/// Abort multipart upload
///
/// Cancel an ongoing multipart upload, clean up uploaded parts.
///
/// # Parameters
///
/// - `client`: S3 client instance
/// - `bucket`: Storage bucket name
/// - `key`: Object key name
/// - `upload_id`: Multipart upload ID
///
/// # Use Cases
///
/// Should call this function to clean up resources when upload fails or is interrupted
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
        .map_err(|e| S3Error::MultipartError(format!("Failed to abort multipart upload: {}", e)))?;
    Ok(())
}

/// List incomplete multipart uploads
///
/// List all incomplete multipart upload tasks in a bucket.
///
/// # Parameters
///
/// - `client`: S3 client instance
/// - `bucket`: Storage bucket name
/// - `prefix`: Optional object key prefix filter
///
/// # Returns
///
/// List of incomplete multipart upload information
///
/// # Use Cases
///
/// - Monitor upload progress
/// - Clean up long-running incomplete uploads
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
        .map_err(|e| S3Error::MultipartError(format!("Failed to list multipart uploads: {}", e)))?;

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

/// Multipart upload information
///
/// Records information about incomplete multipart upload tasks
#[derive(Debug)]
pub struct MultipartUploadInfo {
    /// Object key name
    pub key: String,

    /// Multipart upload ID
    pub upload_id: String,

    /// Initialization time
    pub initiated: Option<String>,
}