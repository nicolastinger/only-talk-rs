use rbatis::{crud, impl_select};
use rbatis::rbdc::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct ChatMessageRecord {
    pub msg_id: Option<i64>,
    pub nano_id: Option<String>,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
    pub msg: Option<String>,
    pub msg_type: Option<String>,
    pub send_user: Uuid,
    pub recv_user: Uuid
}

crud!(ChatMessageRecord {});

impl_select!(ChatMessageRecord {select_chat_by_limit(send_user: Uuid, recv_user: Uuid, start: u32, end: u32) => r#"`where ((send_user = #{send_user} and recv_user = #{recv_user}) or (send_user = #{recv_user} and recv_user = #{send_user}))
  and is_del = false order by created_at limit #{start} offset #{end}`"#
});
