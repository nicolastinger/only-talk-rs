use anyhow::anyhow;
use rbatis::RBatis;
use rbatis::rbdc::Uuid;
use crate::module::chat_msg_mod::entity::chat_message_record::ChatMessageRecord;
use crate::utils::http_response::CommonResponseRef;

pub async fn get_chat_by_limit(rb: &RBatis, account: Option<String>) -> Result<String, anyhow::Error> {
    let account = Some(r#"01965d95-0ffc-7d23-911e-5313485fb9be"#.to_string());
    let uuid = account.ok_or(anyhow!("账号序列化失败"))?.parse::<Uuid>()?;
    let friend_uuid = uuid.clone();
    let res = ChatMessageRecord::select_chat_by_limit(rb,uuid,friend_uuid,0, 10).await?;

    Ok(CommonResponseRef::<Vec<ChatMessageRecord>>::success_json(&res)?)
}

