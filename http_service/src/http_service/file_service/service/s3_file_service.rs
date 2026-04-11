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

use entity::config_str::{OSS_TYPE_ALIYUN, OSS_TYPE_AWS, OSS_TYPE_MINIO, DEFAULT_MAX_FILE_SIZE};
use entity::models::file_entity::file_upload_record::FileUploadRecord;
use entity::utils::time::get_now_time_stamp_as_millis;

use crate::http_service::file_service::service::file_service::validate_file_type;

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
    let storage = S3Storage::new(s3_client.clone());

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
            validate_file_type(filename, mime_type.as_deref()).map_err(|e| anyhow!(e))?;

            // 读取文件数据
            let mut file_data = Vec::new();
            let mut file_size: i64 = 0;
            let mut hasher = Sha256::new();

            while let Some(chunk) = field.next().await {
                let data = match chunk {
                    Ok(d) => d,
                    Err(e) => {
                        error!("读取数据块时出错: {}", e);
                        return Err(anyhow!("未知错误"));
                    }
                };

                let new_size = file_size + data.len() as i64;
                if new_size > DEFAULT_MAX_FILE_SIZE {
                    error!("文件大小超出限制: {} > {}", new_size, DEFAULT_MAX_FILE_SIZE);
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

            // 生成S3对象key：按日期+用户+UUID组织
            let s3_key = format!(
                "{}/{}/{}/{}.{}",
                s3_client.config.default_bucket,
                &user_id[..8.min(user_id.len())],
                chrono_like_date_path(),
                uuid_v4,
                extension
            );

            let now = get_now_time_stamp_as_millis()?;
            let oss_type = get_oss_type(&s3_client.config.provider);

            if !file_upload_record_exist.is_empty() {
                warn!("文件已存在(S3): {}", filename);
                let exist_record = file_upload_record_exist[0].clone();
                let exist_file_path = exist_record.file_path.clone().ok_or(anyhow!("文件路径为空"))?;

                // 已有文件，如果原来存本地则重新上传S3，原来就S3则直接复用
                if exist_record.is_oss.unwrap_or(0) == 0 {
                    // 原来是本地文件，上传到S3
                    let local_data = tokio::fs::read(&exist_file_path).await?;
                    let _ = storage.upload(&s3_key, local_data, mime_type.as_deref()).await?;
                }

                let mut file_record = exist_record.clone();
                file_record.is_oss = Some(1);
                file_record.oss_type = Some(oss_type);
                file_record.file_path = Some(s3_key.clone());
                FileUploadRecord::update_by_map(rb, &file_record, value! {"uuid": &file_record.uuid}).await?;
                file_upload_records.push(file_record);
            } else {
                // 上传到S3
                let storage_info = storage
                    .upload_stream(&s3_key, file_data, file_size, mime_type.as_deref())
                    .await
                    .map_err(|e| anyhow!("S3上传失败: {}", e))?;

                info!("S3上传成功: key={}, size={}", storage_info.key, storage_info.size);

                let file_record = FileUploadRecord {
                    id: None,
                    uuid: Some(rbdc::types::uuid::Uuid::from_str(&uuid_v4_str)?),
                    original_name: Some(filename.to_string()),
                    stored_name: Some(s3_key.clone()),
                    file_path: Some(s3_key),
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
    let storage = S3Storage::new(s3_client);
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
    let storage = S3Storage::new(s3_client);
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
        let storage = S3Storage::new(s3_client);
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
