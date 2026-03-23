use rbatis::rbdc::Uuid;
use rbatis::{crud, impl_select, impl_update};
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct UserInfo {
    pub uuid: Option<Uuid>,
    pub gender: Option<u8>,
    pub age: Option<u8>,
    pub birthday: Option<i64>,
    pub note: Option<String>,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub status: Option<u8>,
}

crud!(UserInfo {});

impl_select!(UserInfo{select_by_uuid(uuid: &Uuid) -> Option => "`where uuid = #{uuid} limit 1`"});
impl_update!(UserInfo{update_by_uuid(uuid: &Uuid) => "`where uuid = #{uuid}`"});
