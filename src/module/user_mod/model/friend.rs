use rbatis::crud;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Friend {
    pub uuid: Option<String>
}

crud!(Friend {});

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FriendLink {
    pub uuid: Option<Uuid>,
    pub request_user: Option<String>,
    pub accept_user: Option<String>,
    pub enable: Option<bool>
}

crud!(FriendLink {});
