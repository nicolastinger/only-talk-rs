use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use actix_multipart::Multipart;
use actix_web::HttpResponse;
use anyhow::anyhow;
use common::config_str::{DEFAULT_MAX_FILE_SIZE, USER_FILE_PUBLIC_DIR};
use common::models::file_entity::file_upload_record::FileUploadRecord;
use common::utils::time::get_now_time_stamp_as_millis;
use futures_util::{StreamExt, TryStreamExt};
use tracing::{error, info, warn};
use rbatis::rbdc::rt::fs::File;
use rbatis::rbdc::rt::{AsyncWriteExt, tokio};
use rbatis::{RBatis, rbdc};
use rbs::value;
use s3_service::S3Client;
use s3_service::storage::StorageBackend;
use sha2::{Digest, Sha256};
use uuid::Uuid;
use common::models::file_entity::biz_file_link::BizFileLink;
use crate::http_service::file_service::model::file_type_config::{get_file_type_config, FileTypeConfig};
use crate::http_service::file_service::service::biz_service::get_pub_file_record_by_biz_id;
use crate::http_service::file_service::service::chat_biz_service::get_chat_file_record_by_biz_id;
use crate::http_service::file_service::service::chat_s3_service::download_chat_file_s3;
use crate::utils::http_response::CommonResponseRef;

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



// 确保目录存在
async fn create_upload_dir() -> std::io::Result<()> {
    tokio::fs::create_dir_all(USER_FILE_PUBLIC_DIR).await
}

/// 文件类型枚举
enum FileType {
    Image,
    Document,
    Archive,
    Audio,
    Video,
}

/// 根据文件名检测文件类型
fn detect_file_type(filename: &str, config: &FileTypeConfig) -> Option<FileType> {
    let extension = std::path::Path::new(filename)
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    if config.image.extensions.iter().any(|ext| ext == &extension) {
        Some(FileType::Image)
    } else if config.document.extensions.iter().any(|ext| ext == &extension) {
        Some(FileType::Document)
    } else if config.archive.extensions.iter().any(|ext| ext == &extension) {
        Some(FileType::Archive)
    } else if config.audio.extensions.iter().any(|ext| ext == &extension) {
        Some(FileType::Audio)
    } else if config.video.extensions.iter().any(|ext| ext == &extension) {
        Some(FileType::Video)
    } else {
        None
    }
}

/// 保存上传的文件到本地，并创建文件记录
async fn save_uploaded_file(
    rb: &RBatis,
    user_id: &str,
    filename: &str,
    extension: &str,
    mime_type: Option<String>,
    field: &mut actix_multipart::Field,
) -> Result<(FileUploadRecord, PathBuf, i64), anyhow::Error> {
    let uuid_v4 = Uuid::new_v4();
    let uuid_v4_str = uuid_v4.to_string();

    let safe_filename = if !extension.is_empty() {
        format!("{}.{}", Uuid::new_v4(), extension)
    } else {
        uuid_v4_str.clone()
    };

    // 构造完整的保存路径
    let mut filepath = PathBuf::from(USER_FILE_PUBLIC_DIR).join(&safe_filename);

    // 创建本地文件
    let mut file = File::create(&filepath).await?;
    let mut file_size = 0;
    let mut hasher = Sha256::new();

    // 从流中读取文件数据块并写入本地文件
    while let Some(chunk) = field.next().await {
        let data = match chunk {
            Ok(d) => d,
            Err(e) => {
                error!("读取数据块时出错: {}", e);
                return Err(anyhow!("未知错误"));
            }
        };

        // 检查文件大小是否超出限制
        let new_size = file_size + data.len() as i64;
        if new_size > DEFAULT_MAX_FILE_SIZE {
            error!("文件大小超出限制: {} > {}", new_size, DEFAULT_MAX_FILE_SIZE);
            return Err(anyhow!(
                "文件大小超出限制，最大允许 {} 字节",
                DEFAULT_MAX_FILE_SIZE
            ));
        }

        file_size = new_size;

        // 更新哈希计算器
        hasher.update(&data);

        // 异步写入数据块
        if let Err(e) = file.write_all(&data).await {
            error!("写入文件时出错: {}", e);
            return Err(anyhow!(e));
        }
    }

    // 计算最终的哈希值
    let hash_result = hasher.finalize();
    let file_hash = format!("{:x}", hash_result);

    // 查询是否有重复文件（相同哈希值和大小）
    let file_upload_record_exist = FileUploadRecord::select_by_map(
        rb,
        value! {"file_size": file_size, "file_hash": &file_hash},
    )
    .await?;

    info!("上传的文件mime_type: {:?}", mime_type);

    let now = get_now_time_stamp_as_millis()?;
    let mut file_record = FileUploadRecord {
        id: None,
        uuid: Some(rbdc::types::uuid::Uuid::from_str(&uuid_v4_str)?),
        original_name: Some(filename.to_string()),
        stored_name: Some(safe_filename.clone()),
        file_path: Some(filepath.display().to_string()),
        bucket: None,  // 本地存储无bucket
        file_size: Some(file_size),
        mime_type,
        file_hash: Some(file_hash.clone()),
        upload_user_uuid: Some(rbdc::types::uuid::Uuid::from_str(user_id)?),
        upload_time: Some(now),
        status: Some(0),
        description: None,
        download_count: None,
        last_download_time: None,
        is_oss: Some(0),
        oss_type: None,
    };

    if !file_upload_record_exist.is_empty() {
        warn!("文件已存在: {}", filename);
        // 如果文件已存在，则删除刚上传的临时文件
        if let Err(e) = tokio::fs::remove_file(&filepath).await {
            error!("删除重复文件失败: {}", e);
        }

        // 使用已存在的文件记录
        let exist_record = file_upload_record_exist[0].clone();
        let exist_file_path = exist_record.file_path.ok_or(anyhow!("文件路径为空"))?;
        filepath = PathBuf::from(&exist_file_path);
        file_record.uuid = exist_record.uuid;
        file_record.file_path = Some(exist_file_path);
        file_record.file_size = exist_record.file_size;
        file_record.file_hash = exist_record.file_hash;
        file_record.mime_type = exist_record.mime_type;
        file_record.original_name = exist_record.original_name;
        file_record.stored_name = exist_record.stored_name;
    } else {
        // 插入文件记录
        FileUploadRecord::insert(rb, &file_record).await?; 
    }
    Ok((file_record, filepath, file_size))
}

/// 处理图片文件（压缩）
async fn process_image_file(
    _rb: &RBatis,
    _user_id: &str,
    _filename: &str,
    _filepath: &PathBuf,
    _now: i64,
) -> Result<Option<FileUploadRecord>, anyhow::Error> {
    // TODO: 实现图片文件的处理逻辑
    Ok(None)
}

/// 处理文档文件
async fn process_document_file(
    _rb: &RBatis,
    _user_id: &str,
    _filename: &str,
    _filepath: &PathBuf,
    _now: i64,
) -> Result<Option<FileUploadRecord>, anyhow::Error> {
    // TODO: 实现文档文件的处理逻辑（例如：PDF转图片、提取文本等）
    Ok(None)
}

/// 处理压缩文件
async fn process_archive_file(
    _rb: &RBatis,
    _user_id: &str,
    _filename: &str,
    _filepath: &PathBuf,
    _now: i64,
) -> Result<Option<FileUploadRecord>, anyhow::Error> {
    // TODO: 实现压缩文件的处理逻辑（例如：解压预览、文件列表等）
    Ok(None)
}

/// 处理音频文件
async fn process_audio_file(
    _rb: &RBatis,
    _user_id: &str,
    _filename: &str,
    _filepath: &PathBuf,
    _now: i64,
) -> Result<Option<FileUploadRecord>, anyhow::Error> {
    // TODO: 实现音频文件的处理逻辑（例如：转码、提取封面、格式转换等）
    Ok(None)
}

/// 处理视频文件
async fn process_video_file(
    _rb: &RBatis,
    _user_id: &str,
    _filename: &str,
    _filepath: &PathBuf,
    _now: i64,
) -> Result<Option<FileUploadRecord>, anyhow::Error> {
    // TODO: 实现视频文件的处理逻辑（例如：转码、提取截图、压缩等）
    Ok(None)
}

/**
 * 处理文件上传请求
 * @param payload: Multipart，包含所有表单字段和文件
 * @return Vec<(原始文件记录,处理后的文件记录)>，处理后的文件记录为Option
 */
pub async fn upload_file_local(
    rb: &RBatis,
    user_id: String,
    mut payload: Multipart,
) -> Result<Vec<FileUploadRecord>, anyhow::Error> {
    // 确保上传目录存在
    if let Err(e) = create_upload_dir().await {
        eprintln!("无法创建上传目录: {}", e);
        return Err(anyhow!("无法创建目录"));
    }

    let config = get_file_type_config()?;
    let mut file_upload_records = Vec::<FileUploadRecord>::new();

    // 遍历 multipart/form-data 中的每个字段
    while let Some(mut field) =
        payload.try_next().await.map_err(|e| anyhow!("无法获取字段: {}", e))?
    {
        // 检查这个字段是否是一个文件（通过 content-disposition 的 filename）
        let content_disposition = field.content_disposition().clone();

        // 仅处理带有 filename 的字段，即文件
        if let Some(filename) = content_disposition.get_filename() {
            // 获取文件扩展名
            let extension = std::path::Path::new(filename)
                .extension()
                .and_then(std::ffi::OsStr::to_str)
                .unwrap_or("");

            // 验证文件类型
            let mime_type = field.content_type().map(|ct| ct.essence_str().to_string());
            
            // 如果客户端没有提供 MIME 类型，则根据文件扩展名推断
            let mime_type = mime_type.or_else(|| {
                infer_mime_from_extension(filename)
            });
            
            validate_file_type(filename, mime_type.as_deref()).map_err(|e| anyhow!(e))?;

            // 保存上传的文件
            let (original_record, filepath, _file_size) =
                save_uploaded_file(rb, &user_id, filename, extension, mime_type, &mut field).await?;

            // 获取当前时间戳
            let now = get_now_time_stamp_as_millis()?;

            // 根据文件类型进行不同的处理
            let _processed_record = if let Some(file_type) = detect_file_type(filename, &config) {
                match file_type {
                    FileType::Image => {
                        process_image_file(rb, &user_id, filename, &filepath, now).await?
                    }
                    FileType::Document => {
                        process_document_file(rb, &user_id, filename, &filepath, now).await?
                    }
                    FileType::Archive => {
                        process_archive_file(rb, &user_id, filename, &filepath, now).await?
                    }
                    FileType::Audio => {
                        process_audio_file(rb, &user_id, filename, &filepath, now).await?
                    }
                    FileType::Video => {
                        process_video_file(rb, &user_id, filename, &filepath, now).await?
                    }
                }
            } else {
                None
            };

            file_upload_records.push(original_record);
        }
    }

    Ok(file_upload_records)
}

/**
 * 验证文件是否为有效的文件类型
 * @param file_name: 文件名
 * @param mime_type: 文件的MIME类型
 */
pub fn validate_file_type(file_name: &str, mime_type: Option<&str>) -> Result<(), String> {
    let config = get_file_type_config().map_err(|e| e.to_string())?;

    // 收集所有支持的扩展名
    let all_extensions: Vec<&String> = vec![
        config.image.extensions.iter(),
        config.document.extensions.iter(),
        config.archive.extensions.iter(),
        config.audio.extensions.iter(),
        config.video.extensions.iter(),
    ]
    .into_iter()
    .flatten()
    .collect();

    // 检查文件扩展名
    let file_extension = file_name.split('.').next_back().map(|s| s.to_lowercase()).unwrap_or_default();

    if !all_extensions.iter().any(|ext| ext.as_str() == file_extension.as_str()) {
        return Err(format!(
            "不支持的文件格式: {}. 支持的格式: {:?}",
            file_extension,
            all_extensions
        ));
    }

    // 检查MIME类型
    if let Some(mime) = mime_type {
        let all_mime_types: Vec<&String> = vec![
            config.image.mime_types.iter(),
            config.document.mime_types.iter(),
            config.archive.mime_types.iter(),
            config.audio.mime_types.iter(),
            config.video.mime_types.iter(),
        ]
        .into_iter()
        .flatten()
        .collect();

        if !all_mime_types.iter().any(|mt| mt.as_str() == mime) {
            return Err(format!(
                "不支持的MIME类型: {}. 支持的类型: {:?}",
                mime,
                all_mime_types
            ));
        }
    }

    Ok(())
}

/**
 * 记录用户下载文件操作
 * @param file_id: 文件id
 * @param user_id: 用户id
 */
pub async fn record_file_download(

) -> Result<(), anyhow::Error> {
    // TODO
    Ok(())
}

/**
 * 通过文件id获取文件详情
 * @param file_id: 文件id
 */
pub async fn get_file_record_by_id(
    rb: &RBatis,
    file_id: &str,
) -> Result<FileUploadRecord, anyhow::Error> {
    let file_id = rbdc::types::uuid::Uuid::from_str(file_id)?;
    let mut file_record = FileUploadRecord::select_by_map(rb, value! {"uuid": &file_id})
        .await?
        .pop()
        .ok_or(anyhow!("文件不存在"))?;

    // 更新文件下载次数
    file_record.download_count = Option::from(file_record.download_count.unwrap_or(0) + 1);
    file_record.last_download_time = Option::from(get_now_time_stamp_as_millis()?);
    FileUploadRecord::update_by_map(rb, &file_record, value! {"uuid": &file_id}).await?;
    Ok(file_record)
}

/**
 * 通过文件路径获取文件，公开文件
 * @param file_path: 文件路径
 */
pub async fn get_file_by_path(file_path: &str) -> Result<Option<File>, anyhow::Error> {
    // 检查文件是否以公开路径开头
    let is_pub = file_path.starts_with(USER_FILE_PUBLIC_DIR);
    if !is_pub {
        return Err(anyhow!("文件路径错误"));
    }

    // 检查文件是否存在
    if !tokio::fs::try_exists(file_path).await.unwrap_or(false) {
        return Ok(None);
    }

    // 获取文件元数据以检查文件大小
    let metadata = tokio::fs::metadata(file_path).await.map_err(|e| anyhow!(e))?;

    // 如果文件大小为0，则返回None
    if metadata.len() == 0 {
        return Ok(None);
    }

    let file = File::open(file_path).await;
    match file {
        Ok(file) => Ok(Some(file)),
        Err(e) => {
            // 对于除文件不存在或空文件之外的其他错误，应该返回错误
            Err(anyhow!(e))
        }
    }
}

/// 单个文件下载
pub async fn download_pub_file_by_id(
    rb: &RBatis, 
    s3_client: Option<Arc<S3Client>>,
    biz_id: String, 
    file_id: String
) -> Result<HttpResponse, anyhow::Error> {
    // 1. 获取业务信息
    info!("biz_id: {}, file_id: {}", biz_id, file_id);
    // 校验业务id是否存在
    get_pub_file_record_by_biz_id(rb, &biz_id).await?;

    let _biz_id = rbdc::Uuid::from_str(&biz_id)?;
    let _file_id = rbdc::Uuid::from_str(&file_id)?;
    let biz_file_link = BizFileLink::select_by_biz_and_file(rb, &_biz_id, &_file_id).await?.ok_or(anyhow!("文件不存在"))?;

    let preview_file_id = biz_file_link.file_id;
    let origin_file_id = biz_file_link.origin_file_id;
    
    let mut flag = false;

    if let Some(preview_file_id) = preview_file_id {
        if preview_file_id.to_string() == file_id {
            flag = true;
        }
    }
    
    if let Some(origin_file_id) = origin_file_id {
        if origin_file_id.to_string() == file_id {
            flag = true;
        }
    }
    if !flag {
        return Err(anyhow!("文件不存在"));
    }
    
    // 2. 获取文件信息
    let file_record = get_file_record_by_id(rb, &file_id).await?;
    
    // 3. 从S3下载
    let s3_client = s3_client.ok_or(anyhow!("S3客户端未初始化"))?;
    if file_record.is_oss.unwrap_or(0) != 1 {
        return Err(anyhow!("文件不是S3存储，无法下载"));
    }
    
    let file_vec = if let Some(ref bucket) = file_record.bucket {
        let storage = s3_service::storage::S3Storage::with_bucket(s3_client.clone(), bucket.clone());
        storage.download(&file_record.file_path.ok_or(anyhow!("文件路径为空"))?)
            .await
            .map_err(|e| anyhow!("S3下载失败: {}", e))?
    } else {
        let storage = s3_service::storage::S3Storage::with_bucket(s3_client.clone(), s3_client.config.default_bucket.clone());
        storage.download(&file_record.file_path.ok_or(anyhow!("文件路径为空"))?)
            .await
            .map_err(|e| anyhow!("S3下载失败: {}", e))?
    };
    
    // 4. 返回文件
    Ok(HttpResponse::Ok()
        .content_type(file_record.mime_type.unwrap_or("image/webp".to_string()))
        .insert_header((
            "Content-Disposition",
            format!(
                "attachment; filename={}",
                file_record.original_name.ok_or(anyhow!("文件名称为空"))?
            ),
        ))
        .body(file_vec))
}

/// 公开业务文件下载link
pub async fn download_link_pub_biz(
    rb: &RBatis, 
    s3_client: Option<Arc<S3Client>>, 
    biz_id: String, 
    is_preview: bool
) -> Result<String, anyhow::Error> {
    // 1. 获取业务信息
    let _biz_record = get_pub_file_record_by_biz_id(rb, &biz_id).await?;
    let _biz_id = rbdc::Uuid::from_str(&biz_id)?;
    let biz_file_link = BizFileLink::select_by_biz(rb, &_biz_id).await?;
    let file_ids = match is_preview {
        true => {
            biz_file_link.into_iter().map(|item| item.file_id.unwrap_or_default().to_string()).collect::<Vec<String>>()
        }
        false => {
            biz_file_link.into_iter().map(|item| item.origin_file_id.unwrap_or_default().to_string()).collect::<Vec<String>>()
        }
    };
    if file_ids.is_empty() {
        return Err(anyhow!("文件ID为空"));
    }
    
    // 2. 组建下载链接 - S3 文件返回预签名 URL
    let s3_client = s3_client.ok_or(anyhow!("S3客户端未初始化"))?;
    let mut download_link_vec: Vec<String> = vec![];
    
    for file_id in file_ids.iter() {
        // 获取文件记录
        let file_record = get_file_record_by_id(rb, file_id).await?;
        
        // 检查是否为S3存储
        if file_record.is_oss.unwrap_or(0) != 1 {
            return Err(anyhow!("文件不是S3存储，无法生成下载链接"));
        }
        
        // 生成预签名 URL
        let presigned_url = if let Some(ref bucket) = file_record.bucket {
            let storage = s3_service::storage::S3Storage::with_bucket(s3_client.clone(), bucket.clone());
            storage.presigned_url(
                &file_record.file_path.ok_or(anyhow!("文件路径为空"))?,
                std::time::Duration::from_secs(s3_client.config.presign_expire_seconds),
                s3_service::storage::PresignedMethod::Get,
            ).await.map_err(|e| anyhow!("生成预签名URL失败: {}", e))?
        } else {
            let storage = s3_service::storage::S3Storage::with_bucket(s3_client.clone(), s3_client.config.default_bucket.clone());
            storage.presigned_url(
                &file_record.file_path.ok_or(anyhow!("文件路径为空"))?,
                std::time::Duration::from_secs(s3_client.config.presign_expire_seconds),
                s3_service::storage::PresignedMethod::Get,
            ).await.map_err(|e| anyhow!("生成预签名URL失败: {}", e))?
        };
        download_link_vec.push(presigned_url);
    }
    
    let res = CommonResponseRef::<Vec<String>>::success_json(&download_link_vec)?;
    Ok(res)
}

/// 聊天业务文件下载link
pub async fn download_link_chat_biz(
    rb: &RBatis, 
    s3_client: Option<Arc<S3Client>>,
    uuid: Option<String>, 
    biz_id: String, 
    is_preview: bool
) -> Result<String, anyhow::Error> {
    // 1. 获取业务信息
    let chat_biz_record = get_chat_file_record_by_biz_id(rb, &biz_id).await?;
    // 2. 校验文件权限
    let user_id = uuid.ok_or(anyhow!("用户ID为空"))?;
    let user_id = rbdc::types::uuid::Uuid::from_str(&user_id)?;
    let created_by = chat_biz_record.created_by.ok_or(anyhow!("创建者ID为空"))?;
    let recv_user_id = chat_biz_record.receiver.ok_or(anyhow!("接收者ID为空"))?;
    if created_by != user_id && recv_user_id != user_id {
        return Err(anyhow!("无权限访问"));
    }

    let _biz_id = rbdc::Uuid::from_str(&biz_id)?;
    let biz_file_link = BizFileLink::select_by_biz(rb, &_biz_id).await?;
    let file_ids = match is_preview {
        true => {
            biz_file_link.into_iter().map(|item| item.file_id.unwrap_or_default().to_string()).collect::<Vec<String>>()
        }
        false => {
            biz_file_link.into_iter().map(|item| item.origin_file_id.unwrap_or_default().to_string()).collect::<Vec<String>>()
        }
    };
    if file_ids.is_empty() {
        return Err(anyhow!("文件ID为空"));
    }
    
    // 3. 组建下载链接 - S3 文件返回预签名 URL
    let s3_client = s3_client.ok_or(anyhow!("S3客户端未初始化"))?;
    let mut download_link_vec: Vec<String> = vec![];
    
    for file_id in file_ids.iter() {
        // 获取文件记录
        let file_record = get_file_record_by_id(rb, file_id).await?;
        
        // 检查是否为S3存储
        if file_record.is_oss.unwrap_or(0) != 1 {
            return Err(anyhow!("文件不是S3存储，无法生成下载链接"));
        }
        
        // 生成预签名 URL
        let presigned_url = if let Some(ref bucket) = file_record.bucket {
            let storage = s3_service::storage::S3Storage::with_bucket(s3_client.clone(), bucket.clone());
            storage.presigned_url(
                &file_record.file_path.ok_or(anyhow!("文件路径为空"))?,
                std::time::Duration::from_secs(s3_client.config.presign_expire_seconds),
                s3_service::storage::PresignedMethod::Get,
            ).await.map_err(|e| anyhow!("生成预签名URL失败: {}", e))?
        } else {
            let storage = s3_service::storage::S3Storage::with_bucket(s3_client.clone(), s3_client.config.chat_file_origin_bucket.clone());
            storage.presigned_url(
                &file_record.file_path.ok_or(anyhow!("文件路径为空"))?,
                std::time::Duration::from_secs(s3_client.config.presign_expire_seconds),
                s3_service::storage::PresignedMethod::Get,
            ).await.map_err(|e| anyhow!("生成预签名URL失败: {}", e))?
        };
        download_link_vec.push(presigned_url);
    }
    
    let res = CommonResponseRef::<Vec<String>>::success_json(&download_link_vec)?;
    Ok(res)
}

/// 聊天业务文件下载
pub async fn download_chat_file_by_id(
    rb: &RBatis, 
    s3_client: Option<Arc<S3Client>>,
    uuid: Option<String>, 
    biz_id: String, 
    file_id: String
) -> Result<HttpResponse, anyhow::Error> {
    // 1. 获取业务信息
    info!("biz_id: {}, file_id: {}", biz_id, file_id);
    let chat_biz_record = get_chat_file_record_by_biz_id(rb, &biz_id).await?;
    // 2. 校验文件权限
    let user_id = uuid.ok_or(anyhow!("用户ID为空"))?;
    let user_id = rbdc::types::uuid::Uuid::from_str(&user_id)?;
    let created_by = chat_biz_record.created_by.ok_or(anyhow!("创建者ID为空"))?;
    let recv_user_id = chat_biz_record.receiver.ok_or(anyhow!("接收者ID为空"))?;
    if created_by != user_id && recv_user_id != user_id {
        return Err(anyhow!("无权限访问"));
    }
    // 3. 组装文件id
    let _biz_id = rbdc::Uuid::from_str(&biz_id)?;
    let _file_id = rbdc::Uuid::from_str(&file_id)?;
    let biz_file_link = BizFileLink::select_by_biz_and_file(rb, &_biz_id, &_file_id).await?.ok_or(anyhow!("文件不存在"))?;

    let preview_file_id = biz_file_link.file_id;
    let origin_file_id = biz_file_link.origin_file_id;
    let mut flag = false;

    if let Some(preview_file_id) = preview_file_id {
        if preview_file_id.to_string() == file_id {
            flag = true;
        }
    }

    if let Some(origin_file_id) = origin_file_id {
        if origin_file_id.to_string() == file_id {
            flag = true;
        }
    }
    if !flag {
        return Err(anyhow!("文件不存在"));
    }
    
    // 4. 获取文件信息
    let file_record = get_file_record_by_id(rb, &file_id).await?;
    
    // 5. 从S3下载
    let s3_client = s3_client.ok_or(anyhow!("S3客户端未初始化"))?;
    if file_record.is_oss.unwrap_or(0) != 1 {
        return Err(anyhow!("文件不是S3存储，无法下载"));
    }
    
    let file_vec = download_chat_file_s3(s3_client, &file_record).await?;
    
    // 6. 返回文件
    Ok(HttpResponse::Ok()
        .content_type(file_record.mime_type.unwrap_or("image/webp".to_string()))
        .insert_header((
            "Content-Disposition",
            format!(
                "attachment; filename={}",
                file_record.original_name.ok_or(anyhow!("文件名称为空"))?
            ),
        ))
        .body(file_vec))
}
