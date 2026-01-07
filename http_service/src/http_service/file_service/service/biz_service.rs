use rbatis::rbdc::types::Uuid as RbdcUuid;
use rbatis::RBatis;
use entity::models::file_entity::biz_record::BizRecord;
use entity::utils::time::get_now_time_stamp_as_millis;
use std::str::FromStr;
use entity::models::file_entity::file_upload_record::FileUploadRecord;
use actix_multipart::Multipart;
use anyhow::anyhow;
use uuid::Uuid;
use entity::models::user_entity::basic_user::BasicUser;

/// 创建上传用户头像业务id
pub async fn create_avatar_biz(rb: &RBatis, file_upload_record: FileUploadRecord) -> Result<BizRecord, anyhow::Error> {
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
        file_ids: Some(file_id),
        created_by: Some(user_id),
        created_at: Some(now),
        updated_at: Some(now),
        status: Some(0), // 0-正常
        approve_status: Some(1), // 1-已通过
        biz_type: Some("avatar".to_string()), // 业务类型为头像
        remark: Some(remark),
    };
    
    // 将业务记录插入数据库
    BizRecord::insert(rb, &biz_record).await?;
    Ok(biz_record)
}
