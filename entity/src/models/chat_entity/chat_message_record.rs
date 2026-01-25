use rbatis::rbdc::{Bytes, Uuid};
use rbatis::{RBatis, crud, impl_select};
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

impl_select!(ChatMessageRecord {select_chat_by_limit(send_user: Uuid, recv_user: Uuid, start: u32, size: u32) => r#"where ((send_user = #{send_user} and recv_user = #{recv_user}) or (send_user = #{recv_user} and recv_user = #{send_user}))
  order by created_at limit #{size} offset #{start}"#
});

// 获取最新一条消息
impl_select!(ChatMessageRecord {select_last_by_column(uuid: &Uuid) -> Option => r#"where recv_user = #{uuid} or send_user = #{uuid} order by timestamp desc limit 1"#});

// 获取未读消息，最大9999
impl_select!(ChatMessageRecord {select_unread_by_time(uuid: &Uuid, time: i64) => r#"where (send_user = #{uuid} or recv_user = #{uuid}) and timestamp > #{time} order by timestamp desc limit 9999"#});

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
