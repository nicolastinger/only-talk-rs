use actix_multipart::Multipart;
use entity::utils::time::get_now_time_stamp_as_millis;
use futures_util::StreamExt as _;
use rbatis::RBatis;
use serde::{Deserialize, Serialize};
use serde_json::json;
use log::error;
use std::io::{Read, Write};
use std::{fs};
use std::path::Path;
use std::str::FromStr;
use actix_web::mime::Mime;
use anyhow::anyhow;
use uuid::Uuid;
use rbatis::rbdc::Uuid as rb_uuid;
use entity::models::file_entity::file_upload_record::FileUploadRecord;

/// 保存文件到本地
pub async fn save_file_to_local(
    rb: &RBatis,
    biz_id: String,
    file: &mut Multipart,
) -> Result<FileUploadRecord, anyhow::Error> { 
    // TODO 检查是否存在业务
    let file_upload_record = FileUploadRecord {
        id: None,
        uuid: None,
        original_name: None,
        stored_name: None,
        file_path: None,
        file_size: None,
        mime_type: None,
        file_hash: None,
        upload_user_uuid: None,
        upload_time: None,
        status: None,
        description: None,
        download_count: None,
        last_download_time: None,
        is_oss: None,
        oss_type: None,
    };
    
    Ok(file_upload_record)
}

