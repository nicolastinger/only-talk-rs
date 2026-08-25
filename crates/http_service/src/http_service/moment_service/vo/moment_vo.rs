use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct MomentVO {
    pub uuid: String,
    pub author_uuid: String,
    pub username: Option<String>,
    pub icon: Option<String>,
    pub content: String,
    pub visibility: i32,
    pub image_count: i64,
    pub like_count: i64,
    pub comment_count: i64,
    pub liked_by_me: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize, Debug)]
pub struct MomentListVO {
    pub total: u32,
    pub list: Vec<MomentVO>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MomentCommentVO {
    pub id: String,
    pub moment_uuid: String,
    pub author_uuid: String,
    pub username: Option<String>,
    pub icon: Option<String>,
    pub content: String,
    pub created_at: i64,
}

#[derive(Serialize, Debug)]
pub struct MomentCommentListVO {
    pub total: u32,
    pub list: Vec<MomentCommentVO>,
}

#[derive(Deserialize, Debug)]
pub struct MomentRow {
    pub uuid: Option<String>,
    pub author_uuid: Option<String>,
    pub content: Option<String>,
    pub visibility: Option<i32>,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
    pub username: Option<String>,
    pub icon: Option<String>,
    pub image_count: Option<i64>,
    pub like_count: Option<i64>,
    pub comment_count: Option<i64>,
    pub liked_by_me: Option<i64>,
}

#[derive(Deserialize, Debug)]
pub struct MomentCommentRow {
    pub id: Option<String>,
    pub moment_uuid: Option<String>,
    pub author_uuid: Option<String>,
    pub content: Option<String>,
    pub created_at: Option<i64>,
    pub username: Option<String>,
    pub icon: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct CountRow {
    pub count: i64,
}
