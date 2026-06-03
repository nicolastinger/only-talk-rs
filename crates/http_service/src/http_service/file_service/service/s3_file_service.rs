use std::sync::Arc;
use std::str::FromStr;

use actix_multipart::Multipart;
use anyhow::anyhow;
use futures_util::{StreamExt, TryStreamExt};
use rbatis::{RBatis, rbdc};
use rbs::value;
use sha2::{Digest, Sha256};
use s3_service::S3Client;
use s3_service::storage::{S3Storage, StorageBackend};
use tracing::{error, info, warn};
use uuid::Uuid;

use common::config_str::{OSS_TYPE_ALIYUN, OSS_TYPE_AWS, OSS_TYPE_MINIO, DEFAULT_MAX_FILE_SIZE};
use common::models::file_entity::file_upload_record::FileUploadRecord;
use common::utils::time::get_now_time_stamp_as_millis;

use crate::http_service::file_service::service::file_service::validate_file_type;

/// 根据文件扩展名推断 MIME 类型
fn infer_mime_from_extension(filename: &str) -> Option<String> {
    let extension = std::path::Path::new(filename)
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or("")
        .to_lowercase();
    
    match extension.as_str() {
        // 图片类型
        "jpg" | "jpeg" => Some("image/jpeg".to_string()),
        "png" => Some("image/png".to_string()),
        "gif" => Some("image/gif".to_string()),
        "webp" => Some("image/webp".to_string()),
        "bmp" => Some("image/bmp".to_string()),
        "svg" => Some("image/svg+xml".to_string()),
        "ico" => Some("image/x-icon".to_string()),
        
        // 文档类型
        "pdf" => Some("application/pdf".to_string()),
        "doc" => Some("application/msword".to_string()),
        "docx" => Some("application/vnd.openxmlformats-officedocument.wordprocessingml.document".to_string()),
        "xls" => Some("application/vnd.ms-excel".to_string()),
        "xlsx" => Some("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".to_string()),
        "ppt" => Some("application/vnd.ms-powerpoint".to_string()),
        "pptx" => Some("application/vnd.openxmlformats-officedocument.presentationml.presentation".to_string()),
        "txt" => Some("text/plain".to_string()),
        "csv" => Some("text/csv".to_string()),
        "html" | "htm" => Some("text/html".to_string()),
        "xml" => Some("application/xml".to_string()),
        "json" => Some("application/json".to_string()),
        
        // 压缩文件
        "zip" => Some("application/zip".to_string()),
        "rar" => Some("application/x-rar-compressed".to_string()),
        "7z" => Some("application/x-7z-compressed".to_string()),
        "tar" => Some("application/x-tar".to_string()),
        "gz" => Some("application/gzip".to_string()),
        
        // 音频文件
        "mp3" => Some("audio/mpeg".to_string()),
        "wav" => Some("audio/wav".to_string()),
        "ogg" => Some("audio/ogg".to_string()),
        "flac" => Some("audio/flac".to_string()),
        "aac" => Some("audio/aac".to_string()),
        
        // 视频文件
        "mp4" => Some("video/mp4".to_string()),
        "avi" => Some("video/x-msvideo".to_string()),
        "mkv" => Some("video/x-matroska".to_string()),
        "mov" => Some("video/quicktime".to_string()),
        "wmv" => Some("video/x-ms-wmv".to_string()),
        "webm" => Some("video/webm".to_string()),
        
        _ => None,
    }
}

/// 获取provider对应的oss_type值
fn get_oss_type(provider: &s3_service::config::S3Provider) -> i32 {
    match provider {
        s3_service::config::S3Provider::MinIO => OSS_TYPE_MINIO,
        s3_service::config::S3Provider::AliyunOSS => OSS_TYPE_ALIYUN,
        s3_service::config::S3Provider::AwsS3 => OSS_TYPE_AWS,
    }
}

/// 上传文件到S3
pub async fn upload_file_s3(
    rb: &RBatis,
    user_id: String,
    mut payload: Multipart,
    s3_client: Arc<S3Client>,
) -> Result<Vec<FileUploadRecord>, anyhow::Error> {
    let bucket = s3_client.config.default_bucket.clone();
    let storage = S3Storage::with_bucket(s3_client.clone(), bucket.clone());

    let mut file_upload_records = Vec::<FileUploadRecord>::new();

    while let Some(mut field) =
        payload.try_next().await.map_err(|e| anyhow!("无法获取字段: {}", e))?
    {
        let content_disposition = field.content_disposition().clone();

        if let Some(filename) = content_disposition.get_filename() {
            let extension = std::path::Path::new(filename)
                .extension()
                .and_then(std::ffi::OsStr::to_str)
                .unwrap_or("");

            let mime_type = field.content_type().map(|ct| ct.essence_str().to_string());
            
            // 如果客户端没有提供 MIME 类型，则根据文件扩展名推断
            let mime_type = mime_type.or_else(|| {
                infer_mime_from_extension(filename)
            });
            
            validate_file_type(filename, mime_type.as_deref()).map_err(|e| anyhow!(e))?;

            // 读取文件数据
            let mut file_data = Vec::new();
            let mut file_size: i64 = 0;
            let mut hasher = Sha256::new();

            while let Some(chunk) = field.next().await {
                let data = match chunk {
                    Ok(d) => d,
                    Err(e) => {
                        error!("error reading data chunk: {}", e);
                        return Err(anyhow!("未知错误"));
                    }
                };

                let new_size = file_size + data.len() as i64;
                if new_size > DEFAULT_MAX_FILE_SIZE {
                    error!("file size exceeds limit: {} > {}", new_size, DEFAULT_MAX_FILE_SIZE);
                    return Err(anyhow!(
                        "文件大小超出限制，最大允许 {} 字节",
                        DEFAULT_MAX_FILE_SIZE
                    ));
                }

                file_size = new_size;
                hasher.update(&data);
                file_data.extend_from_slice(&data);
            }

            let hash_result = hasher.finalize();
            let file_hash = format!("{:x}", hash_result);

            // 查询重复文件
            let file_upload_record_exist = FileUploadRecord::select_by_map(
                rb,
                rbs::value! {"file_size": file_size, "file_hash": &file_hash},
            )
            .await?;

            let uuid_v4 = Uuid::new_v4();
            let uuid_v4_str = uuid_v4.to_string();

            // 生成S3对象key：按日期+用户+UUID组织（不再包含bucket前缀）
            let s3_key = format!(
                "{}/{}/{}.{}",
                &user_id[..8.min(user_id.len())],
                chrono_like_date_path(),
                uuid_v4,
                extension
            );

            let now = get_now_time_stamp_as_millis()?;
            let oss_type = get_oss_type(&s3_client.config.provider);

            if !file_upload_record_exist.is_empty() {
                warn!("file already exists (S3): {}", filename);
                let exist_record = file_upload_record_exist[0].clone();
                let exist_file_path = exist_record.file_path.clone().ok_or(anyhow!("文件路径为空"))?;

                // 已有文件，如果原来存本地则重新上传S3，原来就S3则直接复用
                if exist_record.is_oss.unwrap_or(0) == 0 {
                    // 原来是本地文件，上传到S3
                    let local_data = tokio::fs::read(&exist_file_path).await?;
                    let _ = storage.upload(&s3_key, local_data, mime_type.as_deref()).await?;
                    
                    let mut file_record = exist_record.clone();
                    file_record.is_oss = Some(1);
                    file_record.oss_type = Some(oss_type);
                    file_record.stored_name = Some(s3_key.clone());
                    file_record.file_path = Some(s3_key.clone());
                    // file_path 和 stored_name 保持不变
                    FileUploadRecord::update_by_map(rb, &file_record, value! {"uuid": &file_record.uuid}).await?;
                    file_upload_records.push(file_record);
                    continue;
                }

                // 原来就是S3文件，直接复用
                file_upload_records.push(exist_record);
            } else {
                // 上传到S3
                let storage_info = storage
                    .upload_stream(&s3_key, file_data, file_size, mime_type.as_deref())
                    .await
                    .map_err(|e| anyhow!("S3 upload failed: {}", e))?;

                info!("S3 upload successful: key={}, size={}", storage_info.key, storage_info.size);

                let file_record = FileUploadRecord {
                    id: None,
                    uuid: Some(rbdc::types::uuid::Uuid::from_str(&uuid_v4_str)?),
                    original_name: Some(filename.to_string()),
                    stored_name: Some(s3_key.clone()),
                    file_path: Some(s3_key),
                    bucket: Some(bucket.clone()),
                    file_size: Some(file_size),
                    mime_type,
                    file_hash: Some(file_hash),
                    upload_user_uuid: Some(rbdc::types::uuid::Uuid::from_str(&user_id)?),
                    upload_time: Some(now),
                    status: Some(0),
                    description: None,
                    download_count: None,
                    last_download_time: None,
                    is_oss: Some(1),
                    oss_type: Some(oss_type),
                };

                FileUploadRecord::insert(rb, &file_record).await?;
                file_upload_records.push(file_record);
            }
        }
    }

    Ok(file_upload_records)
}

/// 从S3下载文件
pub async fn download_file_s3(
    s3_client: Arc<S3Client>,
    file_record: &FileUploadRecord,
) -> Result<Vec<u8>, anyhow::Error> {
    let s3_key = file_record.file_path.as_ref().ok_or(anyhow!("S3对象key为空"))?;
    
    // 使用默认桶（普通文件都存储在默认桶中）
    let storage = S3Storage::with_bucket(s3_client.clone(), s3_client.config.default_bucket.clone());
    let data = storage
        .download(s3_key)
        .await
        .map_err(|e| anyhow!("S3下载失败: {}", e))?;
    Ok(data)
}

/// 从S3生成预签名下载URL
pub async fn get_s3_presigned_download_url(
    s3_client: Arc<S3Client>,
    file_record: &FileUploadRecord,
) -> Result<String, anyhow::Error> {
    let s3_key = file_record.file_path.as_ref().ok_or(anyhow!("S3对象key为空"))?;
    let presign_expire_seconds = s3_client.config.presign_expire_seconds;
    
    // 使用默认桶（普通文件都存储在默认桶中）
    let storage = S3Storage::with_bucket(s3_client.clone(), s3_client.config.default_bucket.clone());
    let url = storage
        .presigned_url(
            s3_key,
            std::time::Duration::from_secs(presign_expire_seconds),
            s3_service::storage::PresignedMethod::Get,
        )
        .await
        .map_err(|e| anyhow!("生成预签名URL失败: {}", e))?;
    Ok(url)
}

/// 删除S3上的文件
pub async fn delete_file_s3(
    s3_client: Arc<S3Client>,
    file_record: &FileUploadRecord,
) -> Result<(), anyhow::Error> {
    if file_record.is_oss.unwrap_or(0) == 1 {
        let s3_key = file_record.file_path.as_ref().ok_or(anyhow!("S3对象key为空"))?;
        
        // 使用默认桶（普通文件都存储在默认桶中）
        let storage = S3Storage::with_bucket(s3_client.clone(), s3_client.config.default_bucket.clone());
        storage
            .delete(s3_key)
            .await
            .map_err(|e| anyhow!("S3删除失败: {}", e))?;
    }
    Ok(())
}

/// 生成类似日期的路径（YYYY/MM/DD）
fn chrono_like_date_path() -> String {
    let now = std::time::SystemTime::now();
    let duration = now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let secs = duration.as_secs();
    // 简单的日期计算（不用chrono依赖）
    let days = secs / 86400;
    // 从1970-01-01开始
    let year = 1970 + (days / 365);
    let day_of_year = days % 365;
    let month = 1 + day_of_year / 30;
    let day = 1 + day_of_year % 30;
    format!("{}/{:02}/{:02}", year, month.min(12), day.min(28))
}
