use std::path::PathBuf;
use std::str::FromStr;
use actix_multipart::Multipart;
use actix_web::{HttpResponse, Responder};
use anyhow::anyhow;
use futures_util::{StreamExt, TryStreamExt};
use log::{error, info};
use rbatis::{rbdc, RBatis};
use rbatis::rbdc::rt::fs::File;
use rbatis::rbdc::rt::{tokio, AsyncWriteExt};
use uuid::Uuid;
use sha2::{Sha256, Digest};
use entity::config_str::{USER_FILE_PUBLIC_DIR, DEFAULT_MAX_FILE_SIZE};
use entity::models::file_entity::file_upload_record::FileUploadRecord;
use entity::utils::time::get_now_time_stamp_as_millis;

// 确保目录存在
async fn create_upload_dir() -> std::io::Result<()> {
    tokio::fs::create_dir_all(USER_FILE_PUBLIC_DIR).await
}

/**
 * 处理文件上传请求
 * @param payload: Multipart，包含所有表单字段和文件
 */
pub async fn upload_file_local(rb: &RBatis,user_id: String, mut payload: Multipart) -> Result<Vec<FileUploadRecord>, anyhow::Error> {
    // 确保上传目录存在
    if let Err(e) = create_upload_dir().await {
        eprintln!("无法创建上传目录: {}", e);
        return Err(anyhow!("无法创建目录"));
    }
    
    let mut file_upload_records = Vec::<FileUploadRecord>::new();

    // 遍历 multipart/form-data 中的每个字段
    while let Some(mut field) = payload.try_next().await.map_err(|e| anyhow!("无法获取字段: {}", e))? {
        // 检查这个字段是否是一个文件（通过 content-disposition 的 filename）
        let content_disposition = field.content_disposition().clone();

        // 仅处理带有 filename 的字段，即文件
        if let Some(filename) = content_disposition.get_filename() {
            // 使用UUID-v4生成唯一文件名，同时保留原始文件扩展名
            let extension = std::path::Path::new(filename)
                .extension()
                .and_then(std::ffi::OsStr::to_str)
                .unwrap_or("");
            
            // 验证文件类型
            let mime_type = field.content_type().map(|ct| ct.essence_str().to_string());
            validate_file_type(filename, mime_type.as_deref())
                .map_err(|e| anyhow!(e))?;

            let uuid_v4= Uuid::new_v4();
            let uuid_v4_str = uuid_v4.to_string();

            let safe_filename = if !extension.is_empty() {
                format!("{}.{}", Uuid::new_v4(), extension)
            } else {
                uuid_v4_str.clone()
            };
            
            // 构造完整的保存路径
            let filepath = PathBuf::from(USER_FILE_PUBLIC_DIR).join(&safe_filename);

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
                    return Err(anyhow!("文件大小超出限制，最大允许 {} 字节", DEFAULT_MAX_FILE_SIZE));
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

            let now = get_now_time_stamp_as_millis()?;
            // 计算最终的哈希值
            let hash_result = hasher.finalize();
            let file_hash = format!("{:x}", hash_result);

            let file_upload_record = FileUploadRecord {
                id: None,
                uuid: Some(rbdc::types::uuid::Uuid::from_str(&uuid_v4_str)?),
                original_name: Some(filename.to_string()),
                stored_name: Some(safe_filename),
                file_path: Some(filepath.display().to_string()),
                file_size: Some(file_size),
                mime_type,
                file_hash: Some(file_hash),
                upload_user_uuid: Some(rbdc::types::uuid::Uuid::from_str(&user_id)?),
                upload_time: Some(now),
                status: Some(0),
                description: None,
                download_count: None,
                last_download_time: None,
                is_oss: Some(0),
                oss_type: None,
            };
            FileUploadRecord::insert(rb, &file_upload_record).await?;

            file_upload_records.push(file_upload_record);
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
    // 检查文件扩展名
    let valid_extensions = [
        // 图片格式
        "jpg", "jpeg", "png", "gif", "webp", "bmp",
        // 文档格式
        "txt", "pdf", "doc", "docx", "xls", "xlsx",
        // 压缩格式
        "zip", "rar", "7z",
        // 音频格式
        "mp3", "wav", "flac", "aac", "ogg", "m4a",
        // 视频格式
        "mp4", "avi", "mkv", "mov", "wmv", "flv", "webm",
        // 代码格式
        "json", "xml",
        // 其他格式 
        "md"
    ];
    let file_extension = file_name
        .split('.')
        .last()
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    if !valid_extensions.contains(&file_extension.as_str()) {
        return Err(format!(
            "不支持的文件格式: {}. 支持的格式: {}",
            file_extension,
            valid_extensions.join(", ")
        ));
    }

    // 检查MIME类型
    if let Some(mime) = mime_type {
        let valid_mime_types = [
            // 图片MIME类型
            "image/jpeg", "image/jpg", "image/png",
            "image/gif", "image/webp",
            // 压缩MIME类型
            "application/zip", 
            // 音频MIME类型
            "audio/mpeg", "audio/wav", "audio/x-flac", "audio/aac", "audio/ogg", "audio/x-m4a",
            // 视频MIME类型
            "video/mp4",  "video/webm",
            // 代码和文本MIME类型
            "application/json", "application/xml", "text/markdown"
        ];

        if !valid_mime_types.contains(&mime) {
            return Err(format!(
                "不支持的MIME类型: {}. 支持的类型: {}",
                mime,
                valid_mime_types.join(", ")
            ));
        }
    }

    Ok(())
}