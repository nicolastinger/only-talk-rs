use common::models::user_entity::basic_user::BasicUser;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Validate, Serialize, Deserialize)]
pub struct SignInBasicUserDTO {
    #[validate(required(message = "需要输入id"), length(min = 5, message = "账号长度必须大于5"))]
    pub account: Option<String>,
    #[validate(
        required(message = "需要输入密码"),
        regex(
            path = "common::utils::validators::PASSWORD_REGEX",
            message = "密码必须为14位以上的字母或数字"
        )
    )]
    pub password: Option<String>,
    #[validate(required(message = "需要输入平台"), length(min = 2, message = "平台长度必须大于5"))]
    pub platform: Option<String>,
    #[validate(
        required(message = "需要输入设备指纹"),
        length(min = 16, message = "设备指纹长度必须大于16")
    )]
    pub device_fingerprint: Option<String>,
}

impl SignInBasicUserDTO {
    pub fn to_basic_user(self) -> BasicUser {
        BasicUser {
            uuid: None,
            username: None,
            account: self.account,
            icon: None,
            info: None,
            password: self.password,
            registration_status: None,
        }
    }
}
