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
    /// 是否还有更多消息(新增字段, Option 向后兼容, 老客户端可忽略)
    pub has_more: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UnreadCountVO {
    pub group_uuid: String,
    pub unread_count: i64,
    /// 群成员已读游标，用于按游标拉取未读消息
    pub last_read_msg_id: i64,
}
