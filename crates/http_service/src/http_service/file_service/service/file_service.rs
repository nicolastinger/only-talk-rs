use std::str::FromStr;
use std::sync::Arc;

use actix_web::HttpResponse;
use anyhow::anyhow;
use common::models::file_entity::biz_file_link::BizFileLink;
use common::models::file_entity::file_upload_record::FileUploadRecord;
use common::utils::time::get_now_time_stamp_as_millis;
use rbatis::{RBatis, rbdc};
use rbs::value;
use s3_service::S3Client;
use s3_service::storage::StorageBackend;
use tracing::info;

use crate::http_service::file_service::model::file_type_config::get_file_type_config;
use crate::http_service::file_service::service::biz_service::get_pub_file_record_by_biz_id;
use crate::http_service::file_service::service::chat_biz_service::get_chat_file_record_by_biz_id;
use crate::http_service::file_service::service::chat_s3_service::download_chat_file_s3;
use crate::utils::http_response::CommonResponseRef;

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
    let file_extension =
        file_name.split('.').next_back().map(|s| s.to_lowercase()).unwrap_or_default();

    if !all_extensions.iter().any(|ext| ext.as_str() == file_extension.as_str()) {
        return Err(format!(
            "不支持的文件格式: {}. 支持的格式: {:?}",
            file_extension, all_extensions
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
            return Err(format!("不支持的MIME类型: {}. 支持的类型: {:?}", mime, all_mime_types));
        }
    }

    Ok(())
}

/**
 * 记录用户下载文件操作
 * @param file_id: 文件id
 * @param user_id: 用户id
 */
pub async fn record_file_download() -> Result<(), anyhow::Error> {
    // TODO 待实现
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

/// 单个文件下载
pub async fn download_pub_file_by_id(
    rb: &RBatis,
    s3_client: Arc<S3Client>,
    biz_id: String,
    file_id: String,
) -> Result<HttpResponse, anyhow::Error> {
    // 1. 获取业务信息
    info!("biz_id: {}, file_id: {}", biz_id, file_id);
    // 校验业务id是否存在
    get_pub_file_record_by_biz_id(rb, &biz_id).await?;

    let _biz_id = rbdc::Uuid::from_str(&biz_id)?;
    let _file_id = rbdc::Uuid::from_str(&file_id)?;
    let biz_file_link = BizFileLink::select_by_biz_and_file(rb, &_biz_id, &_file_id)
        .await?
        .ok_or(anyhow!("文件不存在"))?;

    let preview_file_id = biz_file_link.file_id;
    let origin_file_id = biz_file_link.origin_file_id;

    let mut flag = false;

    if let Some(preview_file_id) = preview_file_id
        && preview_file_id.to_string() == file_id
    {
        flag = true;
    }

    if let Some(origin_file_id) = origin_file_id
        && origin_file_id.to_string() == file_id
    {
        flag = true;
    }
    if !flag {
        return Err(anyhow!("文件不存在"));
    }

    // 2. 获取文件信息
    let file_record = get_file_record_by_id(rb, &file_id).await?;

    // 3. 从S3下载
    if file_record.is_oss.unwrap_or(0) != 1 {
        return Err(anyhow!("文件不是S3存储，无法下载"));
    }

    let file_vec = if let Some(ref bucket) = file_record.bucket {
        let storage =
            s3_service::storage::S3Storage::with_bucket(s3_client.clone(), bucket.clone());
        storage
            .download(&file_record.file_path.ok_or(anyhow!("文件路径为空"))?)
            .await
            .map_err(|e| anyhow!("S3下载失败: {}", e))?
    } else {
        let storage = s3_service::storage::S3Storage::with_bucket(
            s3_client.clone(),
            s3_client.config.default_bucket.clone(),
        );
        storage
            .download(&file_record.file_path.ok_or(anyhow!("文件路径为空"))?)
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
    s3_client: Arc<S3Client>,
    biz_id: String,
    is_preview: bool,
) -> Result<String, anyhow::Error> {
    // 1. 获取业务信息
    let _biz_record = get_pub_file_record_by_biz_id(rb, &biz_id).await?;
    let _biz_id = rbdc::Uuid::from_str(&biz_id)?;
    let biz_file_link = BizFileLink::select_by_biz(rb, &_biz_id).await?;
    let file_ids = match is_preview {
        true => biz_file_link
            .into_iter()
            .map(|item| item.file_id.unwrap_or_default().to_string())
            .collect::<Vec<String>>(),
        false => biz_file_link
            .into_iter()
            .map(|item| item.origin_file_id.unwrap_or_default().to_string())
            .collect::<Vec<String>>(),
    };
    if file_ids.is_empty() {
        return Err(anyhow!("文件ID为空"));
    }

    // 2. 组建下载链接 - 公开桶返回直接URL，其他桶返回预签名URL
    let mut download_link_vec: Vec<String> = vec![];

    for file_id in file_ids.iter() {
        // 获取文件记录
        let file_record = get_file_record_by_id(rb, file_id).await?;

        // 检查是否为S3存储
        if file_record.is_oss.unwrap_or(0) != 1 {
            return Err(anyhow!("文件不是S3存储，无法生成下载链接"));
        }

        let file_path = file_record.file_path.as_ref().ok_or(anyhow!("文件路径为空"))?;

        // 判断是否为公开桶
        let is_pub_bucket = match &file_record.bucket {
            Some(bucket) => {
                bucket == &s3_client.config.user_avatar_bucket
                    || bucket == &s3_client.config.group_avatar_bucket
            }
            None => false,
        };

        let url = if let Some(ref bucket) = file_record.bucket {
            let storage =
                s3_service::storage::S3Storage::with_bucket(s3_client.clone(), bucket.clone());
            if is_pub_bucket {
                storage.public_url(file_path)
            } else {
                storage
                    .presigned_url(
                        file_path,
                        std::time::Duration::from_secs(s3_client.config.presign_expire_seconds),
                        s3_service::storage::PresignedMethod::Get,
                    )
                    .await
                    .map_err(|e| anyhow!("生成预签名URL失败: {}", e))?
            }
        } else {
            let storage = s3_service::storage::S3Storage::with_bucket(
                s3_client.clone(),
                s3_client.config.default_bucket.clone(),
            );
            storage
                .presigned_url(
                    file_path,
                    std::time::Duration::from_secs(s3_client.config.presign_expire_seconds),
                    s3_service::storage::PresignedMethod::Get,
                )
                .await
                .map_err(|e| anyhow!("生成预签名URL失败: {}", e))?
        };
        download_link_vec.push(url);
    }

    let res = CommonResponseRef::<Vec<String>>::success_json(&download_link_vec)?;
    Ok(res)
}

/// 聊天业务文件下载link
pub async fn download_link_chat_biz(
    rb: &RBatis,
    s3_client: Arc<S3Client>,
    uuid: Option<String>,
    biz_id: String,
    is_preview: bool,
) -> Result<String, anyhow::Error> {
    // 1. 获取业务信息
    let chat_biz_record = get_chat_file_record_by_biz_id(rb, &biz_id).await?;
    // 2. 校验文件权限
    let user_id = uuid.ok_or(anyhow!("用户ID为空"))?;
    let user_id = rbdc::types::uuid::Uuid::from_str(&user_id)?;
    let created_by = chat_biz_record.created_by.ok_or(anyhow!("创建者ID为空"))?;
    let recv_user_id = chat_biz_record.receiver.ok_or(anyhow!("接收者ID为空"))?;
    let biz_type = chat_biz_record.biz_type.unwrap_or_default();
    if biz_type != "group_chat" && created_by != user_id && recv_user_id != user_id {
        return Err(anyhow!("无权限访问"));
    }

    // TODO 群聊权限实现

    let _biz_id = rbdc::Uuid::from_str(&biz_id)?;
    let biz_file_link = BizFileLink::select_by_biz(rb, &_biz_id).await?;
    let file_ids = match is_preview {
        true => biz_file_link
            .into_iter()
            .map(|item| item.file_id.unwrap_or_default().to_string())
            .collect::<Vec<String>>(),
        false => biz_file_link
            .into_iter()
            .map(|item| item.origin_file_id.unwrap_or_default().to_string())
            .collect::<Vec<String>>(),
    };
    if file_ids.is_empty() {
        return Err(anyhow!("文件ID为空"));
    }

    // 3. 组建下载链接 - S3 文件返回预签名 URL
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
            let storage =
                s3_service::storage::S3Storage::with_bucket(s3_client.clone(), bucket.clone());
            storage
                .presigned_url(
                    &file_record.file_path.ok_or(anyhow!("文件路径为空"))?,
                    std::time::Duration::from_secs(s3_client.config.presign_expire_seconds),
                    s3_service::storage::PresignedMethod::Get,
                )
                .await
                .map_err(|e| anyhow!("生成预签名URL失败: {}", e))?
        } else {
            let storage = s3_service::storage::S3Storage::with_bucket(
                s3_client.clone(),
                s3_client.config.chat_file_origin_bucket.clone(),
            );
            storage
                .presigned_url(
                    &file_record.file_path.ok_or(anyhow!("文件路径为空"))?,
                    std::time::Duration::from_secs(s3_client.config.presign_expire_seconds),
                    s3_service::storage::PresignedMethod::Get,
                )
                .await
                .map_err(|e| anyhow!("生成预签名URL失败: {}", e))?
        };
        download_link_vec.push(presigned_url);
    }

    let res = CommonResponseRef::<Vec<String>>::success_json(&download_link_vec)?;
    Ok(res)
}

/// 聊天业务文件下载
pub async fn download_chat_file_by_id(
    rb: &RBatis,
    s3_client: Arc<S3Client>,
    uuid: Option<String>,
    biz_id: String,
    file_id: String,
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
    let biz_file_link = BizFileLink::select_by_biz_and_file(rb, &_biz_id, &_file_id)
        .await?
        .ok_or(anyhow!("文件不存在"))?;

    let preview_file_id = biz_file_link.file_id;
    let origin_file_id = biz_file_link.origin_file_id;
    let mut flag = false;

    if let Some(preview_file_id) = preview_file_id
        && preview_file_id.to_string() == file_id
    {
        flag = true;
    }

    if let Some(origin_file_id) = origin_file_id
        && origin_file_id.to_string() == file_id
    {
        flag = true;
    }
    if !flag {
        return Err(anyhow!("文件不存在"));
    }

    // 4. 获取文件信息
    let file_record = get_file_record_by_id(rb, &file_id).await?;

    // 5. 从S3下载
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
