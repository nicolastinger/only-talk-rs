use actix_multipart::Multipart;
use entity::models::chat_entity::file_upload_record::FileUploadRecord;
use entity::utils::time::get_now_time_stamp_as_millis;
use futures_util::StreamExt as _;
use rbatis::RBatis;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::{Read, Write};
use std::fs;
use std::path::Path;
use uuid::Uuid;

/// 保存文件记录到数据库
pub async fn save_file_record(
    rb: &RBatis, 
    payload: &mut Multipart,
    user_uuid: Uuid
) -> Result<String, anyhow::Error> {
    // 创建上传目录
    
    Err(anyhow::Error::msg("没有找到上传的文件"))
}

