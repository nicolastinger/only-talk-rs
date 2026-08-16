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

#[cfg(test)]
mod tests {
    use common::models::user_entity::basic_user::BasicUser;
    use common::models::user_entity::user_info::UserInfo;
    use rbatis::rbdc::Uuid;

    use super::*;

    #[test]
    fn from_merges_user_info_and_basic_user() {
        let uuid = Uuid("uuid-1".to_string());
        let user_info = UserInfo {
            uuid: Some(uuid.clone()),
            gender: Some(2),
            age: Some(30),
            birthday: Some(1234567890),
            note: None,
            created_at: Some(1),
            updated_at: Some(2),
            phone: Some("13800138000".to_string()),
            email: Some("a@b.com".to_string()),
            address: Some("beijing".to_string()),
            status: Some(0),
        };
        let basic_user = BasicUser {
            uuid: Some(uuid.clone()),
            username: Some("nick".to_string()),
            account: Some("acct001".to_string()),
            icon: Some("icon-1".to_string()),
            info: Some("hello".to_string()),
            email: None,
            password: None,
        };

        let vo = UserInfoVO::from((user_info, basic_user));

        assert_eq!(vo.uuid, Some(uuid));
        assert_eq!(vo.username.as_deref(), Some("nick"));
        assert_eq!(vo.account.as_deref(), Some("acct001"));
        assert_eq!(vo.icon.as_deref(), Some("icon-1"));
        assert_eq!(vo.info.as_deref(), Some("hello"));
        assert_eq!(vo.gender, Some(2));
        assert_eq!(vo.age, Some(30));
        assert_eq!(vo.birthday, Some(1234567890));
        assert_eq!(vo.phone.as_deref(), Some("13800138000"));
        assert_eq!(vo.email.as_deref(), Some("a@b.com"));
        assert_eq!(vo.address.as_deref(), Some("beijing"));
        assert_eq!(vo.status, Some(0));
    }

    #[test]
    fn from_handles_all_none_fields() {
        let user_info = UserInfo {
            uuid: None,
            gender: None,
            age: None,
            birthday: None,
            note: None,
            created_at: None,
            updated_at: None,
            phone: None,
            email: None,
            address: None,
            status: None,
        };
        let basic_user = BasicUser {
            uuid: None,
            username: None,
            account: None,
            icon: None,
            info: None,
            email: None,
            password: None,
        };

        let vo = UserInfoVO::from((user_info, basic_user));
        assert_eq!(vo.uuid, None);
        assert_eq!(vo.username, None);
        assert_eq!(vo.gender, None);
        assert_eq!(vo.status, None);
    }
}
