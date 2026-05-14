use common::models::user_entity::basic_user::BasicUser;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Clone, Deserialize, Serialize, Debug, Validate)]
pub struct SignUpBasicUserDTO {
    #[validate(required(message = "需要输入名称"), length(min = 5, message = "账号长度必须大于5"))]
    pub username: Option<String>,
    #[validate(required(message = "需要输入账号"), length(min = 5, message = "账号长度必须大于5"))]
    pub account: Option<String>,
    pub icon: Option<String>,
    pub info: Option<String>,
    #[validate(
        required(message = "需要输入密码"),
        regex(
            path = "common::utils::validators::PASSWORD_REGEX",
            message = "密码必须包含大小写字母和数字,并且长度在14以上"
        )
    )]
    pub password: Option<String>,
}

impl SignUpBasicUserDTO {
    pub fn to_basic_user(self) -> BasicUser {
        BasicUser {
            uuid: None,
            username: self.username,
            account: self.account,
            icon: self.icon,
            info: self.info,
            password: self.password,
        }
    }
}
