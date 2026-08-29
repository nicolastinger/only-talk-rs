use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct AnnouncementVO {
    pub uuid: String,
    pub title: String,
    pub content: String,
    pub content_type: i32,
    pub start_at: i64,
    pub end_at: i64,
    pub created_at: i64,
    pub is_read: bool,
    pub read_count: i64,
}

#[derive(Serialize, Debug)]
pub struct AnnouncementListVO {
    pub total: u32,
    pub list: Vec<AnnouncementVO>,
}

#[derive(Serialize, Debug)]
pub struct AnnouncementReadUserVO {
    pub uuid: String,
    pub username: Option<String>,
    pub icon: Option<String>,
    pub created_at: i64,
}

#[derive(Serialize, Debug)]
pub struct AnnouncementReadUserListVO {
    pub total: u32,
    pub list: Vec<AnnouncementReadUserVO>,
}

#[derive(Deserialize, Debug)]
pub struct AnnouncementRow {
    pub uuid: Option<String>,
    pub title: Option<String>,
    pub content: Option<String>,
    pub content_type: Option<i32>,
    pub start_at: Option<i64>,
    pub end_at: Option<i64>,
    pub created_at: Option<i64>,
    pub is_read: Option<i64>,
    pub read_count: Option<i64>,
}

#[derive(Deserialize, Debug)]
pub struct AnnouncementReadUserRow {
    pub uuid: Option<String>,
    pub username: Option<String>,
    pub icon: Option<String>,
    pub created_at: Option<i64>,
}

#[derive(Deserialize, Debug)]
pub struct CountRow {
    pub count: i64,
}
