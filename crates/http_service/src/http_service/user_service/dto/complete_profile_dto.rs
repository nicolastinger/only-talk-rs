use serde::{Deserialize, Serialize};
use validator::Validate;

/// 两步注册第二步: 凭注册会话 token 补全账号、用户名与密码
#[derive(Clone, Deserialize, Serialize, Debug, Validate)]
pub struct CompleteProfileDTO {
    #[validate(
        required(message = "需要输入注册会话token"),
        length(min = 1, message = "注册会话token不能为空")
    )]
    pub reg_token: Option<String>,
    #[validate(
        required(message = "需要输入邮箱"),
        regex(path = "common::utils::validators::EMAIL_REGEX", message = "邮箱格式不正确")
    )]
    pub email: Option<String>,
    #[validate(required(message = "需要输入账号"), length(min = 5, message = "账号长度必须大于5"))]
    pub account: Option<String>,
    #[validate(
        required(message = "需要输入密码"),
        regex(
            path = "common::utils::validators::PASSWORD_REGEX",
            message = "密码必须为14位以上的字母或数字"
        )
    )]
    pub password: Option<String>,
    #[validate(required(message = "需要输入用户名"), length(min = 1, message = "用户名不能为空"))]
    pub username: Option<String>,
    pub icon: Option<String>,
    pub info: Option<String>,
}
