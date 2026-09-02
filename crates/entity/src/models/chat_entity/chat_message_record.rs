use rbatis::executor::Executor;
use rbatis::rbdc::{Bytes, Uuid};
use rbatis::{RBatis, crud};
use rbs::value;
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct ChatMessageRecord {
    pub id: Option<i64>,
    pub nano_id: Option<String>,
    pub timestamp: Option<i64>,
    pub raw: Bytes,
    pub text_type: Option<u32>,
    pub send_user: Uuid,
    pub recv_user: Uuid,
}

crud!(ChatMessageRecord {});

impl ChatMessageRecord {
    #[rbatis::py_sql("select * from chat_message_record where ((send_user = #{send_user} and recv_user = #{recv_user}) or (send_user = #{recv_user} and recv_user = #{send_user})) order by created_at limit #{size} offset #{start}")]
    async fn select_chat_by_limit(rb: &dyn Executor, send_user: Uuid, recv_user: Uuid, start: u32, size: u32) -> Vec<ChatMessageRecord> {}

    // 获取最新一条消息
    #[rbatis::py_sql("select * from chat_message_record where recv_user = #{uuid} or send_user = #{uuid} order by timestamp desc limit 1")]
    async fn select_last_by_column_inner(rb: &dyn Executor, uuid: &Uuid) -> Vec<ChatMessageRecord> {}

    pub async fn select_last_by_column(
        rb: &dyn Executor,
        uuid: &Uuid,
    ) -> rbatis::Result<Option<ChatMessageRecord>> {
        Ok(Self::select_last_by_column_inner(rb, uuid).await?.into_iter().next())
    }

    // 获取未读消息，最大9999
    #[rbatis::py_sql("select * from chat_message_record where (send_user = #{uuid} or recv_user = #{uuid}) and timestamp > #{time} order by timestamp desc limit 9999")]
    async fn select_unread_by_time(rb: &dyn Executor, uuid: &Uuid, time: i64) -> Vec<ChatMessageRecord> {}
}

// rbatis::raw_sql!(chat_message_recordraw_insert(rb: &dyn Executor, nano_id: String, created_at: i64, send_user: Uuid, recv_user: Uuid, raw: Vec<u8>, msg_type: u32)  -> Result<rbs::Value, rbatis::Error> =>
// "INSERT INTO public.chat_message_record
// (nano_id, created_at, send_user, recv_user,raw, text_type)
// VALUES(?, ?, ?, ?, ?, ?);"
// );

pub async fn raw_insert(
    rbatis: &RBatis,
    chat_message_record: ChatMessageRecord,
) -> Result<(), rbatis::Error> {
    let bytes = value!(chat_message_record.raw);
    rbatis
        .exec(
            "INSERT INTO public.chat_message_record
(nano_id, timestamp, send_user, recv_user,raw, text_type)
VALUES($1,$2,$3,$4,$5,$6)",
            vec![
                value!(chat_message_record.nano_id),
                value!(chat_message_record.timestamp),
                value!(chat_message_record.send_user),
                value!(chat_message_record.recv_user),
                bytes,
                value!(chat_message_record.text_type),
            ],
        )
        .await?;
    Ok(())
}
