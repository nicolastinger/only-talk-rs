use std::fs;
use actix_web::{Responder, HttpResponse};
use actix_multipart::Multipart;
use futures::{TryStreamExt, StreamExt};
use tokio::{fs::File, io::AsyncWriteExt};
use std::path::PathBuf;
use anyhow::anyhow;
use rbatis::RBatis;
use tokio::io::AsyncReadExt;
use uuid::Uuid;
use entity::config_str::USER_FILE_PUBLIC_DIR;
use http_service::http_service::file_service::service::biz_service::{create_avatar_biz, get_pub_file_record_by_biz_id};
use http_service::http_service::file_service::service::file_service::{get_file_by_path, get_file_record_by_id, upload_file_local};
use http_service::http_service::user_service::service::user_service::update_user_avatar;
use http_service::utils::http_response::{CommonResponseNoDataRef, CommonResponseRef};

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

/// 用户头像下载
pub async fn download_pub_biz(rb: &RBatis, biz_id: String) -> Result<HttpResponse, anyhow::Error> {
    // 1. 获取业务信息
    let biz_record = get_pub_file_record_by_biz_id(rb, &biz_id).await?;
    let file_ids = biz_record.file_ids.ok_or(anyhow!("文件ID为空"))?;
    if file_ids.is_empty() {
        return Err(anyhow!("文件ID为空"));
    }
    // 按逗号分割文件id
    let file_id_vec: Vec<&str> = file_ids.split(",").collect();
    if file_id_vec.len() > 1 {
        let res = CommonResponseRef::<Vec<&str>>::success_json(
            &file_id_vec,
        )?;
        let result = HttpResponse::Ok().body(res);
        return Ok(result);
    }
    let file_id = file_id_vec[0];
    // 2. 获取文件信息
    let file_record = get_file_record_by_id(rb, &file_id).await?;
    // 3. 返回文件
    let mut file: File = get_file_by_path(&file_record.file_path.ok_or(anyhow!("文件路径为空"))?).await?.ok_or(anyhow!("文件不存在"))?;
    let file_vec: Vec<u8> = {
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).await?;
        buf
    };
    Ok(HttpResponse::Ok()
        .content_type(file_record.mime_type.ok_or(anyhow!("文件类型为空"))?)
        .insert_header(("Content-Disposition", format!("attachment; filename={}", file_record.original_name.ok_or(anyhow!("文件名称为空"))?)))
        .body(file_vec))
}