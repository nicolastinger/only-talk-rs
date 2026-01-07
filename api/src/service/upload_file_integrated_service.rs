use actix_web::{Responder, HttpResponse};
use actix_multipart::Multipart;
use futures::{TryStreamExt, StreamExt};
use tokio::{fs::File, io::AsyncWriteExt};
use std::path::PathBuf;
use anyhow::anyhow;
use rbatis::RBatis;
use uuid::Uuid;
use entity::config_str::USER_FILE_PUBLIC_DIR;
use http_service::http_service::file_service::service::biz_service::create_avatar_biz;
use http_service::http_service::file_service::service::file_service::upload_file_local;
use http_service::http_service::user_service::service::user_service::update_user_avatar;
use http_service::utils::http_response::CommonResponseNoDataRef;

/// 用户头像上传
pub async fn upload_user_avatar(rb: &RBatis, uuid: Option<String>, payload: Multipart) -> Result<String, anyhow::Error> {
    let uuid = uuid.ok_or(anyhow!("用户ID不能为空"))?;
    // 1. 保存文件到本地
    let res = upload_file_local(rb, uuid, payload).await?;
    let first_file = res.into_iter().next().ok_or(anyhow!("未找到上传文件"))?;
    
    // 2. 保存业务信息
    let biz_record = create_avatar_biz(rb, first_file).await?;
    
    // 3. 更新用户头像
    let biz_id = biz_record.uuid.ok_or(anyhow!("用户id为空"))?.to_string();
    let user_id = biz_record.created_by.ok_or(anyhow!("用户id为空"))?;
    update_user_avatar(rb, biz_id, user_id).await?;
    
    Ok(CommonResponseNoDataRef::success_empty())
}


// 确保目录存在
async fn create_upload_dir() -> std::io::Result<()> {
    tokio::fs::create_dir_all(USER_FILE_PUBLIC_DIR).await
}

/**
 * 处理文件上传请求
 * @param payload: Multipart，包含所有表单字段和文件
 */
pub async fn upload_file_local_(mut payload: Multipart) -> impl Responder {
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