use rbatis::crud;
use rbatis::rbdc::Uuid;
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


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FriendLinkInfo {
    pub uuid: Option<Uuid>,
    /// 0-已发起未处理,1-已接受,2-不接受,3-已拒绝再发起,4-拉入黑名单
    pub accept_status: Option<u8>,
    pub create_at_time: Option<i64>,
    pub update_at_time: Option<i64>,
    pub request_message: Option<String>
}

crud!(FriendLinkInfo {});
