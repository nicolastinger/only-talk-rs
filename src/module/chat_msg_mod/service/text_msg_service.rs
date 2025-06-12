use anyhow::anyhow;
use rbatis::RBatis;
use rbatis::rbdc::Uuid;
use crate::module::chat_msg_mod::entity::chat_message_record::ChatMessageRecord;
use crate::module::common::dto::base_page_dto::BasePageDto;
use crate::utils::http_response::CommonResponseRef;

/// 获取聊天记录
pub async fn get_chat_by_limit(rb: &RBatis, uuid: Option<String>, friend_uuid: String, base_page: BasePageDto) -> Result<String, anyhow::Error> {
    let uuid = uuid.ok_or(anyhow!("账号序列化失败"))?.parse::<Uuid>()?;
    let friend_uuid = friend_uuid.parse::<Uuid>()?;
    let start = base_page.page_index.unwrap_or(0);
    let end = base_page.page_index.unwrap_or(10);
    let res = ChatMessageRecord::select_chat_by_limit(rb,uuid,friend_uuid,start, end).await?;

    Ok(CommonResponseRef::<Vec<ChatMessageRecord>>::success_json(&res)?)
}

/// 获取会话列表
pub async fn get_chat_list_link(rb: &RBatis, uuid: Option<String>) -> Result<String, anyhow::Error> {
    let uuid = uuid.ok_or(anyhow!("账号获取失败"))?.parse::<Uuid>()?;
    Ok("".to_string())
}

