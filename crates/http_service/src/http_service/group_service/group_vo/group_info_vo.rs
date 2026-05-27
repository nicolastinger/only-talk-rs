use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct GroupInfoVO {
    pub group_uuid: String,
    pub group_name: String,
    pub avatar: Option<String>,
    pub owner_uuid: String,
    pub description: Option<String>,
    pub max_members: i32,
    pub member_count: i64,
    pub created_at: i64,
    pub updated_at: i64,
    pub status: i16,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GroupListItemVO {
    pub group_uuid: String,
    pub group_name: String,
    pub avatar: Option<String>,
    pub owner_uuid: String,
    pub member_count: i64,
    pub last_msg_time: Option<i64>,
    pub unread_count: i64,
}
