use actix_multipart::Multipart;
use anyhow::anyhow;
use http_service::http_service::file_service::service::biz_service::{
    create_avatar_biz,
};
use http_service::http_service::file_service::service::file_service::{
    upload_file_local,
};
use http_service::http_service::user_service::service::user_service::update_user_avatar;
use http_service::utils::http_response::{CommonResponseNoDataRef};
use rbatis::RBatis;

/// 用户头像上传
pub async fn upload_user_avatar(
    rb: &RBatis,
    uuid: Option<String>,
    payload: Multipart,
) -> Result<String, anyhow::Error> {
    let uuid = uuid.ok_or(anyhow!("用户ID不能为空"))?;
    // 1. 保存文件到本地
    let res = upload_file_local(rb, uuid, payload).await?;
    let (original_record, compressed_record) = res.into_iter().next().ok_or(anyhow!("未找到上传文件"))?;
    // 2. 保存业务信息
    let biz_record = create_avatar_biz(rb, original_record, compressed_record).await?;

    // 3. 更新用户头像
    let biz_id = biz_record.uuid.ok_or(anyhow!("用户id为空"))?.to_string();
    let user_id = biz_record.created_by.ok_or(anyhow!("用户id为空"))?;
    update_user_avatar(rb, biz_id, user_id).await?;

    Ok(CommonResponseNoDataRef::success_empty())
}
