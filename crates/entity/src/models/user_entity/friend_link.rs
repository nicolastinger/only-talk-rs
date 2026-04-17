use rbatis::rbdc::Uuid;
use rbatis::{crud, impl_select, impl_update};
use serde::{Deserialize, Serialize};

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
impl_update!(FriendLink {update_is_del_by_users(request_user: &Uuid, accept_user: &Uuid) => "`where (request_user = #{request_user} and accept_user = #{accept_user}) or (request_user = #{accept_user} and accept_user = #{request_user})`"});
