use rbatis::rbdc::Uuid;
use serde::{Deserialize, Serialize};
use entity::models::user_entity::basic_user::BasicUser;
use entity::models::user_entity::user_info::UserInfo;

#[derive(Deserialize, Serialize, Debug)]
pub struct UserInfoVO {
    uuid: Option<Uuid>,
    username: Option<String>,
    account: Option<String>,
    icon: Option<String>, //头像的超链接
    gender: Option<u8>,   //0-m,1-s,2-男,3-女,4-机器人,5-other,etc
    age: Option<u8>,
    birthday: Option<i64>,
    info: Option<String>, //简介
    created_at: Option<i64>,
    updated_at: Option<i64>,
    phone: Option<String>,
    email: Option<String>,
    address: Option<String>,
    status: Option<u8>,
}

impl From<(UserInfo, BasicUser)> for UserInfoVO {
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
            created_at: sources.0.created_at,
            updated_at: sources.0.updated_at,
            phone: sources.0.phone,
            email: sources.0.email,
            address: sources.0.address,
            status: sources.0.status,
        }
    }
}
