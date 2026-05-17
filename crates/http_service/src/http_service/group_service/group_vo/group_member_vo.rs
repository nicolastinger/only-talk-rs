use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct GroupMemberVO {
    pub user_uuid: String,
    pub role: i16,
    pub nickname: Option<String>,
    pub join_time: i64,
    pub muted: bool,
    pub status: i16,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GroupMemberWithUserInfoVO {
    pub user_uuid: String,
    pub role: i16,
    pub nickname: Option<String>,
    pub join_time: i64,
    pub muted: bool,
    pub status: i16,
    pub username: Option<String>,
    pub avatar: Option<String>,
}
