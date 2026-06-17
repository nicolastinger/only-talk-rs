use crate::http_service::file_service::service::chat_biz_service::get_chat_file_record_by_biz_id;
use crate::http_service::file_service::service::chat_s3_service::upload_chat_origin_file_s3;
use crate::http_service::file_service::vo::biz_file_link_vo::BizFileLinkVO;
use crate::http_service::file_service::vo::biz_record_vo::BizRecordVO;
use crate::utils::http_response::CommonResponseRef;
use actix_multipart::Multipart;
use anyhow::anyhow;
use common::models::file_entity::biz_file_link::BizFileLink;
use common::models::file_entity::biz_record::BizRecord;
use common::utils::time::get_now_time_stamp_as_millis;
use rbatis::{RBatis, rbdc};
use rbs::value;
use s3_service::S3Client;
use std::str::FromStr;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

/// 创建上传用户头像业务id
pub async fn create_avatar_biz(
    rb: &RBatis,
    user_id: rbdc::Uuid,
) -> Result<BizRecord, anyhow::Error> {
    let now = get_now_time_stamp_as_millis()?;
    let uuid_v4 = Uuid::new_v4();
    let uuid_v4_str = uuid_v4.to_string();
    let biz_id = rbatis::rbdc::Uuid::from_str(&uuid_v4_str)?;
    let remark = format!("用户头像上传，用户ID: {}", user_id);

    let biz_record = BizRecord {
        id: None,
        uuid: Some(biz_id),
        biz_name: Some("用户头像上传".to_string()),
        description: Some("用户上传头像文件的业务记录".to_string()),
        created_by: Some(user_id),
        created_at: Some(now),
        updated_at: Some(now),
        status: Some(0),
        approve_status: Some(1),
        biz_type: Some("avatar".to_string()),
        remark: Some(remark),
    };

    BizRecord::insert(rb, &biz_record).await?;
    Ok(biz_record)
}

/// 创建上传群组头像业务id
pub async fn create_group_avatar_biz(
    rb: &RBatis,
    user_id: rbdc::Uuid,
    group_id: rbdc::Uuid,
) -> Result<BizRecord, anyhow::Error> {
    let now = get_now_time_stamp_as_millis()?;
    let uuid_v4 = Uuid::new_v4();
    let uuid_v4_str = uuid_v4.to_string();
    let biz_id = rbatis::rbdc::Uuid::from_str(&uuid_v4_str)?;
    let remark = format!("群组头像上传，用户ID: {}, 群组ID: {}", user_id, group_id);

    let biz_record = BizRecord {
        id: None,
        uuid: Some(biz_id),
        biz_name: Some("群组头像上传".to_string()),
        description: Some("群组上传头像文件的业务记录".to_string()),
        created_by: Some(user_id),
        created_at: Some(now),
        updated_at: Some(now),
        status: Some(0),
        approve_status: Some(1),
        biz_type: Some("group_avatar".to_string()),
        remark: Some(remark),
    };

    BizRecord::insert(rb, &biz_record).await?;
    Ok(biz_record)
}

/**
 * 通过业务id获取文件记录信息
 * @param biz_id: 业务id
 */
pub async fn get_pub_file_record_by_biz_id(
    rb: &RBatis,
    biz_id: &str,
) -> Result<BizRecord, anyhow::Error> {
    // 通过业务id查询文件记录信息
    let biz_id = rbdc::Uuid::from_str(biz_id)?;
    let biz_record =
        BizRecord::select_by_uuid(rb, &biz_id).await?.ok_or(anyhow!("未找到对应的文件记录信息"))?;
    Ok(biz_record)
}

/// 通过业务id补充原文件信息
pub async fn upload_original_file_by_biz_id(
    rb: &RBatis,
    s3_client: Option<Arc<S3Client>>,
    uuid: Option<String>,
    biz_id: String,
    biz_record_type: String,
    preview_id: String,
    payload: Multipart,
) -> Result<String, anyhow::Error> {
    let uuid = uuid.ok_or(anyhow!("用户ID不能为空"))?;
    let user_id = rbdc::Uuid::from_str(&uuid)?;
    // 1. 校验业务id是否存在
    let mut flag = false;
    // 公开文件
    match biz_record_type.as_str() {
        "public" => {
            // 公开业务
            let biz_record = get_pub_file_record_by_biz_id(rb, &biz_id).await?;
            if biz_record.created_by.ok_or(anyhow!("用户业务ID为空"))? == user_id {
                flag = true;
            }
        }
        "chat" => {
            // 聊天业务
            let biz_record = get_chat_file_record_by_biz_id(rb, &biz_id).await?;
            if biz_record.created_by.ok_or(anyhow!("用户业务ID为空"))? == user_id {
                flag = true;
            }
        }
        "private" => {
            // TODO
        }
        _ => return Err(anyhow!("不支持的业务类型")),
    }
    if !flag {
        return Err(anyhow!("业务ID不存在或不属于当前用户"));
    }

    // 2. 使用S3上传
    let s3_client = s3_client.ok_or(anyhow!("S3客户端未初始化"))?;
    if !s3_client.config.enabled {
        return Err(anyhow!("S3服务未启用"));
    }

    info!("uploading original file to S3...");
    let original_record = upload_chat_origin_file_s3(rb, uuid.clone(), payload, s3_client.clone())
        .await
        .map_err(|e| anyhow!("S3上传失败: {}", e))?;
    info!("original file uploaded to S3 successfully");

    let preview_id = rbdc::Uuid::from_str(&preview_id)?;
    // 3. 更新biz_file_link
    let biz_id = rbdc::Uuid::from_str(&biz_id)?;
    let mut biz_file_link = BizFileLink::select_by_biz_and_file(rb, &biz_id, &preview_id)
        .await?
        .ok_or(anyhow!("未找到对应的biz_file_link"))?;
    biz_file_link.origin_file_id = original_record.uuid;
    let biz_file_link_id = biz_file_link.id.ok_or(anyhow!("biz_file_link_id为空"))?;
    BizFileLink::update_by_map(
        rb,
        &biz_file_link,
        value! { "id": &biz_file_link_id, "column": ["origin_file_id"] },
    )
    .await?;

    let biz_file_link_vo = BizFileLinkVO::from_biz_file_link(biz_file_link);
    let biz_file_link_vo_vec = vec![biz_file_link_vo];
    let biz_record_vo = BizRecordVO {
        uuid: Some(biz_id),
        biz_name: None,
        description: None,
        biz_type: None,
        remark: None,
        file_infos: Some(biz_file_link_vo_vec),
    };
    Ok(CommonResponseRef::<BizRecordVO>::success_json(&biz_record_vo)?)
}
