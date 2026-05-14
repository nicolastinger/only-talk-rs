use std::str::FromStr;
use std::sync::Arc;
use actix_multipart::Multipart;
use anyhow::anyhow;
use tracing::{info, warn};
use http_service::http_service::file_service::service::biz_service::{
    create_avatar_biz,
};
use http_service::http_service::file_service::service::file_service::{
    upload_file_local,
};
use http_service::http_service::file_service::service::chat_s3_service::upload_chat_preview_file_s3;
use http_service::http_service::file_service::service::avatar_s3_service::upload_user_avatar_s3;
use http_service::http_service::user_service::service::user_service::update_user_avatar;
use http_service::utils::http_response::CommonResponseRef;
use rbatis::{rbdc, RBatis};
use s3_service::S3Client;
use common::models::file_entity::biz_file_link::BizFileLink;
use common::models::user_entity::friend_link::FriendLink;
use http_service::http_service::file_service::service::chat_biz_service::create_user_chat_biz;
use http_service::http_service::file_service::vo::biz_file_link_vo::BizFileLinkVO;
use http_service::http_service::file_service::vo::biz_record_vo::BizRecordVO;

/// 用户头像上传
pub async fn upload_user_avatar(
    rb: &RBatis,
    uuid: Option<String>,
    payload: Multipart,
    s3_client: Option<Arc<S3Client>>,
) -> Result<String, anyhow::Error> {
    let uuid = uuid.ok_or(anyhow!("用户ID不能为空"))?;
    let user_id = rbdc::Uuid::from_str(&uuid)?;
    
    // 1. 优先使用S3上传头像（如果启用）
    let original_record = if let Some(ref client) = s3_client {
        if client.config.enabled {
            info!("尝试上传头像到S3...");
            match upload_user_avatar_s3(rb, uuid.clone(), payload, client.clone()).await {
                Ok(record) => {
                    info!("头像上传到S3成功");
                    record
                }
                Err(e) => {
                    warn!("S3上传失败: {}，回退到本地存储", e);
                    // S3上传失败，回退到本地存储
                    // 注意：payload已经被消费，需要新的Multipart，这里暂时直接报错
                    return Err(anyhow!("S3上传失败且payload已消费: {}", e));
                }
            }
        } else {
            // S3未启用，使用本地存储
            info!("S3未启用，使用本地存储");
            let res = upload_file_local(rb, uuid, payload).await?;
            res.into_iter().next().ok_or(anyhow!("未找到上传文件"))?
        }
    } else {
        // 没有S3客户端，使用本地存储
        info!("无S3客户端，使用本地存储");
        let res = upload_file_local(rb, uuid, payload).await?;
        res.into_iter().next().ok_or(anyhow!("未找到上传文件"))?
    };
    
    // 2. 保存业务信息
    let biz_record = create_avatar_biz(rb, user_id).await?;
    let biz_file_link = BizFileLink {
        id: None,
        biz_id: biz_record.uuid,
        origin_file_id: None,
        file_id: original_record.uuid,
        is_del: Some(false),
    };
    BizFileLink::insert(rb, &biz_file_link).await?;

    // 3. 更新用户头像
    let biz_id = biz_file_link.biz_id.ok_or(anyhow!("用户id为空"))?.to_string();
    let user_id = biz_record.created_by.ok_or(anyhow!("用户id为空"))?;
    update_user_avatar(rb, biz_id.clone(), user_id).await?;

    Ok(CommonResponseRef::<String>::success_json(&biz_id)?)
}

/// 用户聊天文件上传
pub async fn upload_user_chat_file(
    rb: &RBatis,
    uuid: Option<String>,
    payload: Multipart,
    friend_uuid: String,
    s3_client: Option<Arc<S3Client>>,
) -> Result<String, anyhow::Error> {
    let uuid = uuid.ok_or(anyhow!("用户ID不能为空"))?;
    let user_id = rbdc::Uuid::from_str(&uuid)?;
    // 1. 校验好友关系
    let friend_uuid = rbdc::Uuid::from_str(&friend_uuid)?;

    let friend_link = FriendLink::select_by_last_uuid(rb, &user_id, &friend_uuid).await?;
    let is_friend = friend_link.as_ref()
        .map(|link| !link.is_del.unwrap_or(true))
        .unwrap_or(false);

    if !is_friend {
        return Err(anyhow!("双方不是好友关系，无法发送消息"));
    }
    
    // 2. 优先使用S3上传（如果启用）
    let record = if let Some(ref client) = s3_client {
        if client.config.enabled {
            info!("尝试上传聊天文件到S3...");
            match upload_chat_preview_file_s3(rb, uuid.clone(), payload, client.clone()).await {
                Ok(record) => {
                    info!("聊天文件上传到S3成功");
                    record
                }
                Err(e) => {
                    warn!("S3上传失败: {}，回退到本地存储", e);
                    // S3上传失败，回退到本地存储
                    // 注意：payload已经被消费，需要新的Multipart，这里暂时直接报错
                    return Err(anyhow!("S3上传失败且payload已消费: {}", e));
                }
            }
        } else {
            // S3未启用，使用本地存储
            info!("S3未启用，使用本地存储");
            let res = upload_file_local(rb, uuid, payload).await?;
            res.into_iter().next().ok_or(anyhow!("未找到上传文件"))?
        }
    } else {
        // 没有S3客户端，使用本地存储
        info!("无S3客户端，使用本地存储");
        let res = upload_file_local(rb, uuid, payload).await?;
        res.into_iter().next().ok_or(anyhow!("未找到上传文件"))?
    };
    
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
    // 5. 转换vo
    let biz_record = BizRecordVO::from_chat_biz_record(chat_biz_record, biz_link_vo_vec);
    
    Ok(CommonResponseRef::<BizRecordVO>::success_json(&biz_record)?)
}
