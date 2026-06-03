use std::str::FromStr;
use std::sync::Arc;
use actix_multipart::Multipart;
use anyhow::anyhow;
use tracing::info;
use http_service::http_service::file_service::service::biz_service::{
    create_avatar_biz,
    create_group_avatar_biz,
};

use http_service::http_service::file_service::service::chat_s3_service::upload_chat_preview_file_s3;
use http_service::http_service::file_service::service::avatar_s3_service::upload_user_avatar_s3;
use http_service::http_service::file_service::service::avatar_s3_service::upload_group_avatar_s3;
use http_service::http_service::user_service::service::user_service::update_user_avatar;
use http_service::http_service::group_service::group_service::update_group_avatar_service;
use http_service::utils::http_response::CommonResponseRef;
use rbatis::{rbdc, RBatis};
use s3_service::S3Client;
use common::models::file_entity::biz_file_link::BizFileLink;
use common::models::user_entity::friend_link::FriendLink;
use http_service::http_service::file_service::service::chat_biz_service::create_user_chat_biz;
use http_service::http_service::file_service::vo::biz_file_link_vo::BizFileLinkVO;
use http_service::http_service::file_service::vo::biz_record_vo::BizRecordVO;

/// Upload user avatar
pub async fn upload_user_avatar(
    rb: &RBatis,
    uuid: Option<String>,
    payload: Multipart,
    s3_client: Option<Arc<S3Client>>,
) -> Result<String, anyhow::Error> {
    let uuid = uuid.ok_or(anyhow!("User ID cannot be empty"))?;
    let user_id = rbdc::Uuid::from_str(&uuid)?;
    
    // 1. Upload avatar via S3
    let s3_client = s3_client.ok_or(anyhow!("S3 client not initialized"))?;
    if !s3_client.config.enabled {
        return Err(anyhow!("S3 service not enabled"));
    }
    
    info!("uploading avatar to S3...");
    let original_record = upload_user_avatar_s3(rb, uuid.clone(), payload, s3_client.clone()).await
        .map_err(|e| anyhow!("S3 upload failed: {}", e))?;
    info!("avatar uploaded to S3 successfully");
    
    // 2. Save business info
    let biz_record = create_avatar_biz(rb, user_id).await?;
    let biz_file_link = BizFileLink {
        id: None,
        biz_id: biz_record.uuid,
        origin_file_id: None,
        file_id: original_record.uuid,
        is_del: Some(false),
    };
    BizFileLink::insert(rb, &biz_file_link).await?;

    // 3. Update user avatar
    let biz_id = biz_file_link.biz_id.ok_or(anyhow!("User ID is empty"))?.to_string();
    let user_id = biz_record.created_by.ok_or(anyhow!("User ID is empty"))?;
    update_user_avatar(rb, biz_id.clone(), user_id).await?;

    Ok(CommonResponseRef::<String>::success_json(&biz_id)?)
}

/// Upload user chat file
pub async fn upload_user_chat_file(
    rb: &RBatis,
    uuid: Option<String>,
    payload: Multipart,
    friend_uuid: String,
    s3_client: Option<Arc<S3Client>>,
) -> Result<String, anyhow::Error> {
    let uuid = uuid.ok_or(anyhow!("User ID cannot be empty"))?;
    let user_id = rbdc::Uuid::from_str(&uuid)?;
    // 1. Verify friend relationship
    let friend_uuid = rbdc::Uuid::from_str(&friend_uuid)?;

    let friend_link = FriendLink::select_by_last_uuid(rb, &user_id, &friend_uuid).await?;
    let is_friend = friend_link.as_ref()
        .map(|link| !link.is_del.unwrap_or(true))
        .unwrap_or(false);

    if !is_friend {
        return Err(anyhow!("Not friends with each other, unable to send message"));
    }
    
    // 2. Upload via S3
    let s3_client = s3_client.ok_or(anyhow!("S3 client not initialized"))?;
    if !s3_client.config.enabled {
        return Err(anyhow!("S3 service not enabled"));
    }
    
    info!("uploading chat file to S3...");
    let record = upload_chat_preview_file_s3(rb, uuid.clone(), payload, s3_client.clone()).await
        .map_err(|e| anyhow!("S3 upload failed: {}", e))?;
    info!("chat file uploaded to S3 successfully");
    
    // 3. 保存业务信息
    let chat_biz_record = create_user_chat_biz(rb, user_id, friend_uuid).await?;
    // 4. 保存文件关联信息
    let biz_file_link = BizFileLink {
        id: None,
        biz_id: chat_biz_record.uuid.clone(),
        origin_file_id: None,
        file_id: record.uuid,
        is_del: Some(false),
    };
    BizFileLink::insert(rb, &biz_file_link).await?;
    let biz_link_vo = BizFileLinkVO::from_biz_file_link(biz_file_link);
    let biz_link_vo_vec = vec![biz_link_vo];
    // 5. Convert to VO
    let biz_record = BizRecordVO::from_chat_biz_record(chat_biz_record, biz_link_vo_vec);
    
    Ok(CommonResponseRef::<BizRecordVO>::success_json(&biz_record)?)
}

/// Upload group avatar
pub async fn upload_group_avatar(
    rb: &RBatis,
    uuid: Option<String>,
    group_uuid: String,
    payload: Multipart,
    s3_client: Option<Arc<S3Client>>,
) -> Result<String, anyhow::Error> {
    let uuid = uuid.ok_or(anyhow!("User ID cannot be empty"))?;
    let user_id = rbdc::Uuid::from_str(&uuid)?;
    let group_id = rbdc::Uuid::from_str(&group_uuid)?;

    let s3_client = s3_client.ok_or(anyhow!("S3 client not initialized"))?;
    if !s3_client.config.enabled {
        return Err(anyhow!("S3 service not enabled"));
    }
    
    info!("uploading group avatar to S3...");
    let original_record = upload_group_avatar_s3(rb, uuid.clone(), payload, s3_client.clone()).await
        .map_err(|e| anyhow!("S3 upload failed: {}", e))?;
    info!("group avatar uploaded to S3 successfully");

    let biz_record = create_group_avatar_biz(rb, user_id, group_id).await?;
    let biz_file_link = BizFileLink {
        id: None,
        biz_id: biz_record.uuid,
        origin_file_id: None,
        file_id: original_record.uuid,
        is_del: Some(false),
    };
    BizFileLink::insert(rb, &biz_file_link).await?;

    let biz_id = biz_file_link.biz_id.ok_or(anyhow!("biz_id is empty"))?.to_string();
    update_group_avatar_service(rb, biz_id.clone(), &group_uuid).await?;

    Ok(CommonResponseRef::<String>::success_json(&biz_id)?)
}
