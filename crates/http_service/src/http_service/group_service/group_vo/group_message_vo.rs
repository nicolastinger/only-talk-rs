use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct GroupMessageVO {
    pub nano_id: String,
    pub group_uuid: String,
    pub send_user: String,
    pub timestamp: i64,
    pub raw: Vec<u8>,
    pub msg_type: i16,
    pub recalled: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UnreadCountVO {
    pub group_uuid: String,
    pub unread_count: i64,
}
