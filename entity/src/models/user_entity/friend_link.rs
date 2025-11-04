use rbatis::rbdc::Uuid;
use rbatis::{crud, impl_select};
use serde::{Deserialize, Serialize};

/// 好友关系表
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FriendLink {
    pub uuid: Option<Uuid>,
    pub request_user: Option<Uuid>,
    pub accept_user: Option<Uuid>,
    pub is_del: Option<bool>,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
    pub version: Option<i32>,
}

crud!(FriendLink {});

impl_select!(FriendLink {select_by_last_uuid(uuid:&Uuid, last_uuid:&Uuid) -> Option => "`where (accept_user = #{uuid} and request_user = #{last_uuid}) or (accept_user = #{last_uuid} and request_user = #{uuid}) limit 1`"});
