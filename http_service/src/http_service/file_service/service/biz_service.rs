use std::str::FromStr;
use actix_multipart::Multipart;
use anyhow::anyhow;
use entity::models::file_entity::biz_record::BizRecord;
use entity::models::file_entity::file_upload_record::FileUploadRecord;
use entity::utils::time::get_now_time_stamp_as_millis;
use rbatis::{RBatis, rbdc};
use rbs::value;
use uuid::Uuid;
use entity::models::file_entity::biz_file_link::BizFileLink;
use crate::http_service::file_service::service::chat_biz_service::get_chat_file_record_by_biz_id;
use crate::http_service::file_service::service::file_service::upload_file_local;
use crate::http_service::file_service::vo::biz_file_link_vo::BizFileLinkVO;
use crate::http_service::file_service::vo::biz_record_vo::BizRecordVO;
use crate::http_service::user_service::service::user_service::update_user_avatar;
use crate::utils::http_response::{CommonResponseNoDataRef, CommonResponseRef};

/// 创建上传用户头像业务id
pub async fn create_avatar_biz(
    rb: &RBatis,
    file_upload_record: FileUploadRecord,
) -> Result<BizRecord, anyhow::Error> {
    let now = get_now_time_stamp_as_millis()?;
    let uuid_v4 = Uuid::new_v4();
    let uuid_v4_str = uuid_v4.to_string();
    let biz_id = rbatis::rbdc::Uuid::from_str(&uuid_v4_str)?;
    let user_id = file_upload_record.upload_user_uuid.ok_or(anyhow!("上传用户ID不能为空"))?;
    let file_id = file_upload_record.uuid.ok_or(anyhow!("文件ID不能为空"))?.to_string();
    let remark = format!("用户头像上传，用户ID: {}", user_id);


    let biz_record = BizRecord {
        id: None, // ID由数据库自动生成
        uuid: Some(biz_id),
        biz_name: Some("用户头像上传".to_string()),
        description: Some("用户上传头像文件的业务记录".to_string()),
        created_by: Some(user_id),
        created_at: Some(now),
        updated_at: Some(now),
        status: Some(0),                      // 0-正常
        approve_status: Some(1),              // 1-已通过
        biz_type: Some("avatar".to_string()), // 业务类型为头像
        remark: Some(remark),
    };

    // 将业务记录插入数据库
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
        },
        "chat" => {
            // 聊天业务
            let biz_record = get_chat_file_record_by_biz_id(rb, &biz_id).await?;
            if biz_record.created_by.ok_or(anyhow!("用户业务ID为空"))? == user_id {
                flag = true;
            }
        },
        "private" => {
            // TODO
        }
        _ => return Err(anyhow!("不支持的业务类型"))
    }
    if !flag {
        return Err(anyhow!("业务ID不存在或不属于当前用户"))
    }

    // 2. 保存文件到本地
    let res = upload_file_local(rb, uuid, payload).await?;
    let original_record = res.into_iter().next().ok_or(anyhow!("未找到上传文件"))?;

    let preview_id = rbdc::Uuid::from_str(&preview_id)?;
    // 3. 更新biz_file_link
    let biz_id = rbdc::Uuid::from_str(&biz_id)?;
    let mut biz_file_link = BizFileLink::select_by_biz_and_file(rb, &biz_id, &preview_id).await?.ok_or(anyhow!("未找到对应的biz_file_link"))?;
    biz_file_link.origin_file_id = original_record.uuid;
    let biz_file_link_id = biz_file_link.id.ok_or(anyhow!("biz_file_link_id为空"))?;
    BizFileLink::update_by_map(rb, &biz_file_link, value!{ "id": &biz_file_link_id, "column": ["origin_file_id"] }).await?;

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
