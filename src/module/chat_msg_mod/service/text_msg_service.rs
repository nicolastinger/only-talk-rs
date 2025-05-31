use crate::module::chat_msg_mod::entity::chat_message_record::ChatMessageRecord;

pub async fn get_chat_by_limit() -> Result<String, anyhow::Error> {
    let res = ChatMessageRecord::select_chat_by_limit().await?;
}