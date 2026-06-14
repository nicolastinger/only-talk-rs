use anyhow::anyhow;
use common::config_str::USER_READ_MSG;
use common::models::chat_entity::chat_message_read::ChatMessageRecordRead;
use common::models::chat_entity::chat_message_record::ChatMessageRecord;
use common::utils::redis_utils::get_redis_conn;
use deadpool_redis::redis::AsyncCommands;
use rbatis::RBatis;
use rbatis::rbdc::Uuid;
use rbs::value;
use tracing::info;

use crate::common::dto::base_page_dto::BasePageDTO;
use crate::utils::http_response::{CommonResponseNoDataRef, CommonResponseRef};

/// 获取聊天记录
pub async fn get_chat_by_limit(
    rb: &RBatis,
    uuid: Option<String>,
    friend_uuid: String,
    base_page: BasePageDTO,
) -> Result<String, anyhow::Error> {
    let uuid = uuid.ok_or(anyhow!("账号序列化失败"))?.parse::<Uuid>()?;
    let friend_uuid = friend_uuid.parse::<Uuid>()?;
    let start = base_page.page_num.unwrap_or(0);
    let size = base_page.page_num.unwrap_or(10);
    let res = ChatMessageRecord::select_chat_by_limit(rb, uuid, friend_uuid, start, size).await?;

    let chat = res.first().ok_or(anyhow!("没有数据"))?;
    let vec = chat.raw.clone();
    let str = String::from_utf8(vec.into_inner())?;
    info!("result: {}", str);
    Ok(CommonResponseRef::<Vec<ChatMessageRecord>>::success_json(&res)?)
}

// 用户新增聊天记录
// pub async fn add_user_chat_record(text_msg: TextQuicMsg) -> Result<(), anyhow::Error> {
//     // TODO kafka转发消息ck批量写入
//     let rb = RBATIS_DATABASE.read().await;
//     let rb = rb.as_ref().ok_or(anyhow!("获取连接失败"))?;
//     let chat_msg = ChatMessageRecord::from(text_msg)?;
//     ChatMessageRecord::insert(rb, &chat_msg).await?;
//     Ok(())
// }

/// 获取未读消息
pub async fn get_unread_chat_record(
    rb: &RBatis,
    uuid: Option<String>,
) -> Result<String, anyhow::Error> {
    info!("request received");
    let uuid = uuid.ok_or(anyhow!("账号获取失败"))?.parse::<Uuid>()?;

    let empty_vec = CommonResponseNoDataRef::success_empty();
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
        return Ok(CommonResponseRef::<Vec<ChatMessageRecord>>::success_json(&unread_msg)?);
    }
    // 4、查找已读消息有没有最新消息
    let last_msg_ref = last_msg.as_ref().ok_or(anyhow!("获取最新消息失败"))?;
    let res = read_msg.iter().find(|x| x.nano_id == last_msg_ref.nano_id);
    if res.is_some() {
        info!("request finished");
        return Ok(empty_vec);
    }
    // 5、获取未读消息
    let last_read = read_msg
        .first()
        .ok_or(anyhow!("failed to get read messages"))?
        .nano_id
        .clone()
        .ok_or(anyhow!("failed to get read message timestamp"))?;
    let last_record = ChatMessageRecord::select_by_map(rb, value! {"nano_id": &last_read}).await?;
    if !last_record.is_empty() {
        let last_read = last_record
            .last()
            .ok_or(anyhow!("failed to get read message"))?
            .timestamp
            .ok_or(anyhow!("failed to get read message timestamp"))?;
        let unread_msg = ChatMessageRecord::select_unread_by_time(rb, &uuid, last_read).await?;
        let unread_msg: Vec<ChatMessageRecord> = unread_msg
            .into_iter()
            .map(|mut x| {
                x.id = None;
                x
            })
            .collect();
        info!("request finished");
        return Ok(CommonResponseRef::<Vec<ChatMessageRecord>>::success_json(&unread_msg)?);
    }
    info!("request finished");
    Ok(empty_vec)
}

// 用户新增已读消息
pub async fn add_user_chat_read(
    uuid: Option<String>,
    chat_message_read: Vec<ChatMessageRecordRead>,
) -> Result<String, anyhow::Error> {
    let uuid = uuid.ok_or(anyhow!("账号获取失败"))?;
    let chat_message_read_str = serde_json::to_string(&chat_message_read)?;
    // 写入到redis
    let key = format!("{}{}", USER_READ_MSG, uuid);
    let mut redis = get_redis_conn().await?;
    let res: Result<String, _> = redis.get(&key).await;

    if res.is_err() {
        let _: () = redis.set_ex(&key, chat_message_read_str, 60 * 60 * 24).await?;
    } else {
        let mut last_chat_message_read: Vec<ChatMessageRecordRead> = serde_json::from_str(&res?)?;
        for item in chat_message_read.into_iter() {
            let new_item =
                last_chat_message_read.iter_mut().find(|x| x.send_user == item.send_user);
            if let Some(new_item) = new_item {
                new_item.timestamp = item.timestamp;
                new_item.nano_id = item.nano_id.clone();
                info!("update last_chat_message_read: {:?}", new_item);
            } else {
                last_chat_message_read.push(item)
            }
        }

        let _: () = redis
            .set_ex(&key, serde_json::to_string(&last_chat_message_read)?, 60 * 60 * 24)
            .await?;
    }

    Ok(CommonResponseNoDataRef::success_empty())
}
