use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Validate, Serialize, Deserialize)]
pub struct SignInBasicUserDTO {
    #[validate(required(message = "需要输入id"), length(min = 5, message = "账号长度必须大于5"))]
    pub account: Option<String>,
    #[validate(required(message = "需要输入密码"), regex(path = "crate::utils::validators::PASSWORD_REGEX", message = "密码必须包含大小写字母和数字,并且长度在14以上"))]
    pub password: Option<String>
}