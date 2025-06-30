use crate::common::quic_network_service::models::text_msg::TextQuicMsg;
use crate::module::chat_msg_mod::entity::chat_message_record::{
    chat_message_recordraw_insert, raw_insert, ChatMessageRecord,
};
use crate::module::common::dto::base_page_dto::BasePageDto;
use crate::utils::http_response::CommonResponseRef;
use crate::RBATIS_DATABASE;
use anyhow::anyhow;
use log::info;
use rbatis::executor::RBatisTxExecutor;
use rbatis::rbdc::Uuid;
use rbatis::RBatis;

/// 获取聊天记录
pub async fn get_chat_by_limit(
    rb: &RBatis,
    uuid: Option<String>,
    friend_uuid: String,
    base_page: BasePageDto,
) -> Result<String, anyhow::Error> {
    let uuid = uuid.ok_or(anyhow!("账号序列化失败"))?.parse::<Uuid>()?;
    let friend_uuid = friend_uuid.parse::<Uuid>()?;
    let start = base_page.page_index.unwrap_or(0);
    let size = base_page.page_index.unwrap_or(10);
    let res = ChatMessageRecord::select_chat_by_limit(rb, uuid, friend_uuid, start, size).await?;

    let chat = res.get(0).ok_or(anyhow!("没有数据"))?;
    let vec = chat.raw.clone();    
    let str = String::from_utf8(vec.into_inner())?;
    info!("结果为 {}", str);
    Ok(CommonResponseRef::<Vec<ChatMessageRecord>>::success_json(
        &res,
    )?)
}

/// 获取会话列表
pub async fn get_chat_list_link(
    rb: &RBatis,
    uuid: Option<String>,
) -> Result<String, anyhow::Error> {
    let uuid = uuid.ok_or(anyhow!("账号获取失败"))?.parse::<Uuid>()?;
    Ok("".to_string())
}

/// 用户新增聊天记录
pub async fn add_user_chat_record(text_msg: TextQuicMsg) -> Result<(), anyhow::Error> {
    // TODO kafka转发消息ck批量写入
    let rb = RBATIS_DATABASE.read().await;
    let rb = rb.as_ref().ok_or(anyhow!("获取连接失败"))?;
    let chat_msg = ChatMessageRecord::from(text_msg)?;
    ChatMessageRecord::insert(rb, &chat_msg).await?;
    Ok(())
}
