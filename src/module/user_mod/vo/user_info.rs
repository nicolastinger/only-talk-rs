use rbatis::rbdc::Uuid;
use serde::{Deserialize, Serialize};
use crate::module::user_mod::entity::basic_user::BasicUser;
use crate::module::user_mod::entity::user_info::UserInfo;

#[derive(Deserialize, Serialize, Debug)]
pub struct UserInfoVO {
    uuid: Option<Uuid>,
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

impl From<(UserInfo,BasicUser)> for UserInfoVO {
    fn from(sources: (UserInfo, BasicUser)) -> Self {
        Self {
            uuid: sources.0.uuid,
            username: sources.1.username,
            account: sources.1.account,
            icon: sources.1.icon,
            gender: sources.0.gender,
            age: sources.0.age,
            birthday: sources.0.birthday,
            info: sources.1.info,
            create_at: sources.0.create_at,
            update_at: sources.0.update_at,
            last_login_at: sources.0.last_login_at,
            last_login_equipment: sources.0.last_login_equipment,
            last_login_ipv4: sources.0.last_login_ipv4,
            last_login_ipv6: sources.0.last_login_ipv6,
            phone: sources.0.phone,
            email: sources.0.email,
            address: sources.0.address,
            status: sources.0.status,
        }
    }
}