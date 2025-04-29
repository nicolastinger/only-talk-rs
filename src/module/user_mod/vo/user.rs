use serde::{Deserialize, Serialize};
use crate::module::user_mod::model::basic_user::UserInfo;

#[derive(Deserialize, Serialize, Debug)]
pub struct UserInfoVO {
    username: Option<String>,
    account: Option<String>,
    icon: Option<String>,  //头像的超链接
    gender: Option<u8>,  //0-m,1-s,2-男,3-女,4-机器人,5-other,etc
    age: Option<u8>,
    birthday: Option<i64>,
    info: Option<String>,  //简介
    create_at: Option<i64>,
    update_at: Option<i64>,
    last_login_at: Option<i64>,
    last_login_equipment: Option<String>,  //最后登录的设备
    last_login_ipv4: Option<String>,
    last_login_ipv6: Option<String>,
    phone: Option<String>,
    email: Option<String>,
    address: Option<String>,
    status: Option<u8>
}

impl From<UserInfo> for UserInfoVO {
    fn from(entity: UserInfo) -> Self {
        Self {
            username: entity.username,
            account: entity.account,
            icon: entity.icon,
            gender: entity.gender,
            age: entity.age,
            birthday: entity.birthday,
            info: entity.info,
            create_at: entity.create_at,
            update_at: entity.update_at,
            last_login_at: entity.last_login_at,
            last_login_equipment: entity.last_login_equipment,
            last_login_ipv4: entity.last_login_ipv4,
            last_login_ipv6: entity.last_login_ipv6,
            phone: entity.phone,
            email: entity.email,
            address: entity.address,
            status: entity.status,
        }
    }
}