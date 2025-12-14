use actix_web::{Responder, HttpResponse};
use actix_multipart::Multipart;
use futures::{TryStreamExt, StreamExt};
use tokio::{fs::File, io::AsyncWriteExt};
use std::path::PathBuf;
use uuid::Uuid;
use entity::config_str::USER_FILE_PUBLIC_DIR;


// 确保目录存在
async fn create_upload_dir() -> std::io::Result<()> {
    tokio::fs::create_dir_all(USER_FILE_PUBLIC_DIR).await
}

/**
 * 处理文件上传请求
 * @param payload: Multipart，包含所有表单字段和文件
 */
pub async fn upload_file_local(mut payload: Multipart) -> impl Responder {
    // 确保上传目录存在
    if let Err(e) = create_upload_dir().await {
        eprintln!("无法创建上传目录: {}", e);
        return HttpResponse::InternalServerError().body(format!("无法创建目录: {}", e));
    }

    // 遍历 multipart/form-data 中的每个字段
    while let Some(mut field) = payload.try_next().await.expect("无法获取字段") {
        // 检查这个字段是否是一个文件（通过 content-disposition 的 filename）
        let content_disposition = field.content_disposition().clone();

        // 仅处理带有 filename 的字段，即文件
        if let Some(filename) = content_disposition.get_filename() {
            // 使用UUID-v4生成唯一文件名，同时保留原始文件扩展名
            let extension = std::path::Path::new(filename)
                .extension()
                .and_then(std::ffi::OsStr::to_str)
                .unwrap_or("");
            
            let safe_filename = if !extension.is_empty() {
                format!("{}.{}", Uuid::new_v4(), extension)
            } else {
                Uuid::new_v4().to_string()
            };

            // 构造完整的保存路径
            let filepath = PathBuf::from(USER_FILE_PUBLIC_DIR).join(&safe_filename);

            // 创建本地文件
            let mut file = match File::create(&filepath).await {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("无法创建文件 {}: {}", filepath.display(), e);
                    return HttpResponse::InternalServerError().body("无法创建本地文件");
                }
            };

            // 从流中读取文件数据块并写入本地文件
            while let Some(chunk) = field.next().await {
                let data = match chunk {
                    Ok(d) => d,
                    Err(e) => {
                        eprintln!("读取数据块时出错: {}", e);
                        return HttpResponse::InternalServerError().body("读取文件数据时出错");
                    }
                };

                // 异步写入数据块
                if let Err(e) = file.write_all(&data).await {
                    eprintln!("写入文件时出错: {}", e);
                    return HttpResponse::InternalServerError().body("写入文件时出错");
                }
            }

            println!("✅ 文件已成功保存到: {}", filepath.display());
            return HttpResponse::Ok().body(format!("文件 '{}' 上传成功!", safe_filename));
        }
    }

    // 如果请求中没有找到文件字段
    HttpResponse::BadRequest().body("未找到上传文件")
}