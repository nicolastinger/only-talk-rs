use anyhow::anyhow;
use deadpool_redis::redis::AsyncCommands;
use log::{info};
use rbatis::dark_std::err;
use rbs::value;
use crate::http_service::chat_service::entity::chat_message_read::ChatMessageRecordRead;
use crate::RBATIS_DATABASE;
use crate::utils::global_static_str::USER_READ_MSG;
use crate::utils::redis_utils::get_redis_conn;

/// 用户下线
pub async fn user_offline(uuid: String) -> Result<(), anyhow::Error> {
    // TODO
    let mut redis = get_redis_conn().await?;
    let rb = RBATIS_DATABASE.read().await;
    // 1.设置redis分布式锁，防止用户下线的同时立马上线
    // 2.同步所有redis缓存到数据库，记录用户操作
    // 已读消息从redis中持久化到数据库
    let read_key = format!("{}{}", USER_READ_MSG, uuid);
    let read_record: String = redis.get(&read_key).await?;
    info!("已读消息, 源 {}", read_record);
    let last_chat_message_read: Vec<ChatMessageRecordRead> = serde_json::from_str(&read_record)?;
    info!("已读消息, 转换 {:?}", last_chat_message_read);
    // TODO已读消息有效校验
    let rb = rb.as_ref().ok_or(anyhow!("获取连接失败"))?;
    for item in last_chat_message_read.into_iter() {
        let insert_item = async |e| {
            match ChatMessageRecordRead::insert(rb, &item).await  {
                Ok(_) => {
                },
                Err(x) => {
                    err!("更新已读消息失败 {} {}", e, x);
                }
            }
        };
        match ChatMessageRecordRead::update_by_map(rb, &item, value!{"send_user": &item.send_user, "recv_user": &item.recv_user}).await {
            Ok(d) => {
                if d.rows_affected < 1u64 {
                    insert_item(d.to_string()).await;
                }
            },
            Err(e) => {
                insert_item(e.to_string()).await;
            }
        };
    }

    // 3.清理redis缓存，清理redis锁
    Ok(())
}

/// 用户上线
pub async fn user_online(uuid: String) -> Result<(), anyhow::Error> {
    // TODO
    // 1.设置redis分布式锁，防止用户上线的同时立马下线
    // 2.同步所有数据库到redis缓存
    // 3.清理redis锁
    Ok(())
}