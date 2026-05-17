use common::models::user_entity::basic_user::BasicUser;
use common::models::user_entity::user_info::UserInfo;
use rbatis::rbdc::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct UserInfoVO {
    /// 用户唯一标识符 (UUID)
    pub uuid: Option<Uuid>,
    /// 用户名
    pub username: Option<String>,
    /// 用户账号
    pub account: Option<String>,
    /// 头像的业务 ID
    pub icon: Option<String>,
    /// 用户性别 (0: 未知, 1: 保密, 2: 男, 3: 女, 4: 机器人, 5: 其他)
    pub gender: Option<u8>,
    /// 用户年龄
    pub age: Option<u8>,
    /// 用户生日 (Unix 时间戳，单位：秒)
    pub birthday: Option<i64>,
    /// 用户简介
    pub info: Option<String>,
    /// 用户手机号码
    pub phone: Option<String>,
    /// 用户电子邮箱
    pub email: Option<String>,
    /// 用户地址信息
    pub address: Option<String>,
    /// 用户状态 (0: 正常, 1: 禁用, 2: 注销等)
    pub status: Option<u8>,
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
            phone: sources.0.phone,
            email: sources.0.email,
            address: sources.0.address,
            status: sources.0.status,
        }
    }
}
