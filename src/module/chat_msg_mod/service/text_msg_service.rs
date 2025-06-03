use anyhow::anyhow;
use rbatis::RBatis;
use rbatis::rbdc::Uuid;
use crate::module::chat_msg_mod::entity::chat_message_record::ChatMessageRecord;
use crate::utils::http_response::CommonResponseRef;

pub async fn get_chat_by_limit(rb: &RBatis, uuid: Option<String>) -> Result<String, anyhow::Error> {
    let uuid = uuid.ok_or(anyhow!("账号序列化失败"))?.parse::<Uuid>()?;
    let friend_uuid = uuid.clone();
    let res = ChatMessageRecord::select_chat_by_limit(rb,uuid,friend_uuid,0, 10).await?;

    Ok(CommonResponseRef::<Vec<ChatMessageRecord>>::success_json(&res)?)
}

