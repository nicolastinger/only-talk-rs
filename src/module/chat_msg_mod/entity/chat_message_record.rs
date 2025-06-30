use crate::common::quic_network_service::models::text_msg::TextQuicMsg;
use rbatis::executor::Executor;
use rbatis::rbdc::{Bytes, Uuid};
use rbatis::{crud, impl_select, rbdc, RBatis};
use rbs::value;
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct ChatMessageRecord {
    pub msg_id: Option<i64>,
    pub nano_id: Option<String>,
    pub created_at: Option<i64>,
    pub raw: Bytes,
    pub msg_type: Option<u32>,
    pub send_user: Uuid,
    pub recv_user: Uuid,
}

crud!(ChatMessageRecord {});

impl_select!(ChatMessageRecord {select_chat_by_limit(send_user: Uuid, recv_user: Uuid, start: u32, size: u32) => r#"where ((send_user = #{send_user} and recv_user = #{recv_user}) or (send_user = #{recv_user} and recv_user = #{send_user}))
  order by created_at limit #{size} offset #{start}"#
});

rbatis::raw_sql!(chat_message_recordraw_insert(rb: &dyn Executor, nano_id: String, created_at: i64, send_user: Uuid, recv_user: Uuid, raw: Vec<u8>, msg_type: u32)  -> Result<rbs::Value, rbatis::Error> =>
"INSERT INTO public.chat_message_record
(nano_id, created_at, send_user, recv_user,raw, msg_type)
VALUES(?, ?, ?, ?, ?, ?);"
);

pub async fn raw_insert(
    rbatis: &RBatis,
    chat_message_record: ChatMessageRecord,
) -> Result<(), rbatis::Error> {
    let bytes = value!(chat_message_record.raw);
    rbatis
        .exec(
            "INSERT INTO public.chat_message_record
(nano_id, created_at, send_user, recv_user,raw, msg_type)
VALUES($1,$2,$3,$4,$5,$6)",
            vec![
                value!(chat_message_record.nano_id),
                value!(chat_message_record.created_at),
                value!(chat_message_record.send_user),
                value!(chat_message_record.recv_user),
                bytes,
                value!(chat_message_record.msg_type),
            ],
        )
        .await?;
    Ok(())
}

impl ChatMessageRecord {
    pub fn from(text_quic_msg: TextQuicMsg) -> Result<ChatMessageRecord, anyhow::Error> {
        Ok(ChatMessageRecord {
            msg_id: None,
            nano_id: Some(text_quic_msg.id),
            created_at: Some(text_quic_msg.timestamp),
            raw: Bytes::from(text_quic_msg.raw),
            msg_type: Some(text_quic_msg.text_type as u32),
            send_user: text_quic_msg.send_user.parse::<Uuid>()?,
            recv_user: text_quic_msg.recv_user.parse::<Uuid>()?,
        })
    }
}
