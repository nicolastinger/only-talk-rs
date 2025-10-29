use rbatis::rbdc::Uuid;
use rbatis::{crud, impl_select};
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct UserInfo {
    pub uuid: Option<Uuid>,
    pub gender: Option<u8>, //0-m,1-s,2-男,3-女,4-机器人,5-other,etc
    pub age: Option<u8>,
    pub birthday: Option<i64>,
    pub note: Option<String>, //个人简介
    pub create_at: Option<i64>,
    pub update_at: Option<i64>,
    pub last_login_at: Option<i64>,
    pub last_login_equipment: Option<String>, //最后登录的设备
    pub last_login_ipv4: Option<String>,
    pub last_login_ipv6: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub status: Option<u8>,
}

crud!(UserInfo {});

impl_select!(UserInfo{select_by_uuid(uuid: &Uuid) -> Option => "`where uuid = #{uuid} limit 1`"});
