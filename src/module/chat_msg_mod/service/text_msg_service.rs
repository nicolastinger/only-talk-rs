use crate::common::quic_network_service::models::text_msg::TextQuicMsg;
use crate::module::chat_msg_mod::entity::chat_message_record::{
    raw_insert, ChatMessageRecord,
};
use crate::module::common::dto::base_page_dto::BasePageDto;
use crate::utils::http_response::{CommonResponseNoDataRef, CommonResponseRef};
use crate::RBATIS_DATABASE;
use anyhow::anyhow;
use deadpool_redis::redis::AsyncCommands;
use log::{info, warn};
use rbatis::rbdc::Uuid;
use rbatis::RBatis;
use rbs::value;
use crate::module::chat_msg_mod::entity::chat_message_read::{ ChatMessageRecordRead};
use crate::utils::global_static_str::{REDIS_SPLIT, USER_READ_MSG};
use crate::utils::redis_utils::get_redis_conn;

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

/// 获取未读消息
pub async fn get_unread_chat_record(
    rb: &RBatis,
    uuid: Option<String>,
) -> Result<String, anyhow::Error> {
    let uuid = uuid.ok_or(anyhow!("账号获取失败"))?.parse::<Uuid>()?;
    let empty_vec: Vec<ChatMessageRecord> = vec![];
    let empty_vec = serde_json::to_string(&empty_vec)?;
    // 1、获取最新消息id
    let last_msg = ChatMessageRecord::select_last_by_column(rb, &uuid).await?;
    if last_msg.is_none() {
        return Ok(empty_vec);
    }
    // 2、获取已读消息列表
    let read_msg = ChatMessageRecordRead::select_all_read_by_column(rb, &uuid, 200).await?;
    if read_msg.is_empty() {
       // 3、返回最新消息，最大9999
        let last_read = 0;
        let unread_msg = ChatMessageRecord::select_unread_by_time(rb, &uuid, last_read).await?;
        info!("unread_msg {}", unread_msg.len());
        return Ok(serde_json::to_string(&unread_msg)?);
    }
    // 4、查找已读消息有没有最新消息
    let res = read_msg.iter().find(|x| x.nano_id == last_msg.as_ref().unwrap().nano_id);
    if res.is_some() {
        return Ok(empty_vec);
    }
    // 5、获取未读消息
    let last_read = read_msg.first().ok_or(anyhow!("获取已读消息失败"))?.nano_id.clone().ok_or(anyhow!("获取已读消息时间失败"))?;
    let last_record = ChatMessageRecord::select_by_map(rb, value!{"nano_id": &last_read}).await?;
    if !last_record.is_empty() {
        let last_read = last_record.last().ok_or(anyhow!("获取已读消息失败"))?.timestamp.ok_or(anyhow!("获取已读消息时间失败"))?;
        let unread_msg = ChatMessageRecord::select_unread_by_time(rb, &uuid, last_read).await?;
        let unread_msg: Vec<ChatMessageRecord> = unread_msg.into_iter().map(|mut x| {x.id = None; x}).collect();
        return Ok(serde_json::to_string(&unread_msg)?);  
    }
    Ok(empty_vec)
}

// 用户新增已读消息
pub async fn add_user_chat_read(uuid: Option<String>,
                                chat_message_read: Vec<ChatMessageRecordRead>) -> Result<String, anyhow::Error> {
    let uuid = uuid.ok_or(anyhow!("账号获取失败"))?;
    let chat_message_read_str = serde_json::to_string(&chat_message_read)?;
    // 写入到redis
    let key = format!("{}{}", USER_READ_MSG, uuid);
    let mut redis = get_redis_conn().await?;
    let res = redis.get::<_, String>(&key).await;

    if res.is_err() {
        redis.set_ex::<_, _, ()>(&key, chat_message_read_str, 60 * 60 * 24).await?;
    } else {
        let mut last_chat_message_read: Vec<ChatMessageRecordRead> = serde_json::from_str(&res?)?;
        for item in chat_message_read.into_iter() {
            let new_item = last_chat_message_read.iter_mut().find(|x| x.send_user == item.send_user);
            if new_item.is_some() {
                let new_item = new_item.unwrap();
                new_item.timestamp = item.timestamp;
                new_item.nano_id = item.nano_id.clone();
                info!("update last_chat_message_read: {:?}", new_item);
            } else {
                last_chat_message_read.push(item)
            }
        }

        redis.set_ex::<_, _, ()>(&key, serde_json::to_string(&last_chat_message_read)?, 60 * 60 * 24).await?;
    }

    Ok(CommonResponseNoDataRef::success_empty())
}
