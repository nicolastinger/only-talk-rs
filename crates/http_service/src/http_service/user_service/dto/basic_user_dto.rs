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
            message = "密码必须包含大小写字母和数字,并且长度在14以上"
        )
    )]
    pub password: Option<String>,
    #[validate(required(message = "需要输入平台"), length(min = 2, message = "平台长度必须大于5"))]
    pub platform: Option<String>,
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
        }
    }
}
