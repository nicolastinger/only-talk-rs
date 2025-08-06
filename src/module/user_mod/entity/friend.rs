use rbatis::{crud, impl_select};
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
    pub request_user: Option<Uuid>,
    pub accept_user: Option<Uuid>,
    pub enable: Option<bool>,
    pub created_at: Option<i64>,
}

crud!(FriendLink {});

impl_select!(FriendLink {select_by_last_uuid(uuid:&Uuid, last_uuid:&Uuid) -> Option => "`where (accept_user = #{uuid} and request_user = #{last_uuid}) or (accept_user = #{last_uuid} and request_user = #{uuid}) limit 1`"});

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FriendLinkInfo {
    pub uuid: Option<Uuid>,
    /// 0-已发起未处理,1-已接受,2-不接受,3-已拒绝再发起,4-拉入黑名单
    pub accept_status: Option<u8>,
    pub create_at: Option<i64>,
    pub update_at: Option<i64>,
    pub request_message: Option<String>
}

crud!(FriendLinkInfo {});

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FriendListRecord {
    pub uuid: Option<Uuid>,
    pub create_at: Option<i64>,
    pub update_at: Option<i64>,
    //版本id
    pub version: Option<Uuid>  
}

crud!(FriendListRecord {});
