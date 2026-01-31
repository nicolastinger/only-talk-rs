use std::str::FromStr;
use actix_multipart::Multipart;
use anyhow::anyhow;
use http_service::http_service::file_service::service::biz_service::{
    create_avatar_biz,
};
use http_service::http_service::file_service::service::file_service::{
    upload_file_local,
};
use http_service::http_service::user_service::service::user_service::update_user_avatar;
use http_service::utils::http_response::{CommonResponseNoDataRef, CommonResponseRef};
use rbatis::{rbdc, RBatis};
use entity::models::user_entity::friend_link::FriendLink;
use http_service::http_service::file_service::service::chat_biz_service::create_user_chat_biz;
use http_service::http_service::file_service::vo::biz_record_vo::BizRecordVO;

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

/// 用户聊天文件上传
pub async fn upload_user_chat_file(
    rb: &RBatis,
    uuid: Option<String>,
    payload: Multipart,
    friend_uuid: String,
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
    // 2. 保存文件到本地
    let res = upload_file_local(rb, uuid, payload).await?;
    let (original_record, compressed_record) = res.into_iter().next().ok_or(anyhow!("未找到上传文件"))?;
    // 3. 保存业务信息
    let chat_biz_record = create_user_chat_biz(rb, original_record, compressed_record, friend_uuid).await?;
    // 4. 转换vo
    let biz_record = BizRecordVO::from_chat_biz_record(chat_biz_record);
    
    Ok(CommonResponseRef::<BizRecordVO>::success_json(&biz_record)?)
}
