use rbatis::rbdc::Uuid;
use rbatis::{crud, impl_select};
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct ChatListLink {
    pub id: Option<usize>,
    pub uuid: Uuid,
    pub friend_uuid: Uuid,
    pub created_at: Option<i64>,
    pub enable: Option<bool>,
}

crud!(ChatListLink {});
