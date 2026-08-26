use rbatis::rbdc::Uuid;
use rbatis::{crud, impl_select};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlackList {
    pub uuid: Option<Uuid>,
    pub me_user: Option<Uuid>,
    pub block_user: Option<Uuid>,
    pub created_at: Option<i64>,
    pub version: Option<i32>,
}

crud!(BlackList {});

impl_select!(BlackList {select_by_pair(me_user:&Uuid, block_user:&Uuid) -> Option => "`where me_user = #{me_user} and block_user = #{block_user} limit 1`"});
impl_select!(BlackList {select_by_me_user(me_user:&Uuid) -> Vec => "`where me_user = #{me_user}`"});
