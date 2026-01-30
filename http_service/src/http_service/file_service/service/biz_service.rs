use std::str::FromStr;

use anyhow::anyhow;
use entity::models::file_entity::biz_record::BizRecord;
use entity::models::file_entity::file_upload_record::FileUploadRecord;
use entity::utils::time::get_now_time_stamp_as_millis;
use rbatis::{RBatis, rbdc};
use uuid::Uuid;

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
        file_ids: Some(file_id),
        preview_file_ids: None,
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
