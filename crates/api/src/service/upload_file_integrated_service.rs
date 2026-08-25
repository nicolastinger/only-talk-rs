use std::str::FromStr;
use std::sync::Arc;

use actix_multipart::Multipart;
use anyhow::anyhow;
use common::models::file_entity::biz_file_link::BizFileLink;
use common::models::user_entity::friend_link::FriendLink;
use http_service::http_service::file_service::service::avatar_s3_service::{
    upload_group_avatar_s3, upload_user_avatar_s3,
};
use http_service::http_service::file_service::service::biz_service::{
    create_avatar_biz, create_group_avatar_biz,
};
use http_service::http_service::file_service::service::chat_biz_service::{
    create_group_chat_biz, create_user_chat_biz,
};
use http_service::http_service::file_service::service::chat_s3_service::upload_chat_preview_file_s3;
use http_service::http_service::file_service::vo::biz_file_link_vo::BizFileLinkVO;
use http_service::http_service::file_service::vo::biz_record_vo::BizRecordVO;
use http_service::http_service::group_service::group_service::update_group_avatar_service;
use http_service::http_service::user_service::service::user_service::update_user_avatar;
use http_service::utils::http_response::CommonResponseRef;
use rbatis::{RBatis, rbdc};
use s3_service::S3Client;
use tracing::info;

/// 上传用户头像
pub async fn upload_user_avatar(
    rb: &RBatis,
    uuid: Option<String>,
    payload: Multipart,
    s3_client: Arc<S3Client>,
) -> Result<String, anyhow::Error> {
    let uuid = uuid.ok_or(anyhow!("User ID cannot be empty"))?;
    let user_id = rbdc::Uuid::from_str(&uuid)?;

    info!("正在上传头像到 S3...");
    let original_record = upload_user_avatar_s3(rb, uuid.clone(), payload, s3_client.clone())
        .await
        .map_err(|e| anyhow!("S3 upload failed: {}", e))?;
    info!("头像上传到 S3 成功");

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

/// 上传用户聊天文件
pub async fn upload_user_chat_file(
    rb: &RBatis,
    uuid: Option<String>,
    payload: Multipart,
    friend_uuid: String,
    s3_client: Arc<S3Client>,
) -> Result<String, anyhow::Error> {
    let uuid = uuid.ok_or(anyhow!("User ID cannot be empty"))?;
    let user_id = rbdc::Uuid::from_str(&uuid)?;
    // 1. Verify friend relationship
    let friend_uuid = rbdc::Uuid::from_str(&friend_uuid)?;

    let friend_link = FriendLink::select_by_last_uuid(rb, &user_id, &friend_uuid).await?;
    let is_friend = friend_link.as_ref().map(|link| !link.is_del.unwrap_or(true)).unwrap_or(false);

    if !is_friend {
        return Err(anyhow!("Not friends with each other, unable to send message"));
    }

    // 2. Upload via S3
    info!("正在上传聊天文件到 S3...");
    let record = upload_chat_preview_file_s3(rb, uuid.clone(), payload, s3_client.clone())
        .await
        .map_err(|e| anyhow!("S3 upload failed: {}", e))?;
    info!("聊天文件上传到 S3 成功");

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

/// 上传群聊文件(不校验好友关系)
pub async fn upload_group_chat_file(
    rb: &RBatis,
    uuid: Option<String>,
    payload: Multipart,
    group_uuid: String,
    s3_client: Arc<S3Client>,
) -> Result<String, anyhow::Error> {
    let uuid = uuid.ok_or(anyhow!("User ID cannot be empty"))?;
    let user_id = rbdc::Uuid::from_str(&uuid)?;
    let group_id = rbdc::Uuid::from_str(&group_uuid)?;

    // 通过 S3 上传(群聊不做好友校验)
    info!("正在上传群聊文件到 S3...");
    let record = upload_chat_preview_file_s3(rb, uuid.clone(), payload, s3_client.clone())
        .await
        .map_err(|e| anyhow!("S3 upload failed: {}", e))?;
    info!("群聊文件上传到 S3 成功");

    // 保存业务信息(不校验好友关系)
    let chat_biz_record = create_group_chat_biz(rb, user_id, group_id).await?;
    // 保存文件关联
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
    // 转换为 VO
    let biz_record = BizRecordVO::from_chat_biz_record(chat_biz_record, biz_link_vo_vec);

    Ok(CommonResponseRef::<BizRecordVO>::success_json(&biz_record)?)
}

/// 上传群头像
pub async fn upload_group_avatar(
    rb: &RBatis,
    uuid: Option<String>,
    group_uuid: String,
    payload: Multipart,
    s3_client: Arc<S3Client>,
) -> Result<String, anyhow::Error> {
    let uuid = uuid.ok_or(anyhow!("User ID cannot be empty"))?;
    let user_id = rbdc::Uuid::from_str(&uuid)?;
    let group_id = rbdc::Uuid::from_str(&group_uuid)?;

    info!("正在上传群头像到 S3...");
    let original_record = upload_group_avatar_s3(rb, uuid.clone(), payload, s3_client.clone())
        .await
        .map_err(|e| anyhow!("S3 upload failed: {}", e))?;
    info!("群头像上传到 S3 成功");

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

/// 上传动态广场图片(仅上传文件, 返回 file_id, 由 /moment/create 负责关联)
pub async fn upload_moment(
    rb: &RBatis,
    uuid: Option<String>,
    payload: Multipart,
    s3_client: Arc<S3Client>,
) -> Result<String, anyhow::Error> {
    let uuid = uuid.ok_or(anyhow!("User ID cannot be empty"))?;

    info!("正在上传动态广场图片到 S3...");
    let record = upload_chat_preview_file_s3(rb, uuid, payload, s3_client.clone())
        .await
        .map_err(|e| anyhow!("S3 upload failed: {}", e))?;
    info!("动态广场图片上传到 S3 成功");

    let file_id = record.uuid.ok_or(anyhow!("file id is empty"))?.to_string();
    Ok(CommonResponseRef::<String>::success_json(&file_id)?)
}
