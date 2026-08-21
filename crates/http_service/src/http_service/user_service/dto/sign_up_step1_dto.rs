use serde::{Deserialize, Serialize};
use validator::Validate;

/// 两步注册第一步: 提交邮箱 + 验证码, 验证通过后创建占位用户并下发注册会话 token
#[derive(Clone, Deserialize, Serialize, Debug, Validate)]
pub struct SignUpStep1DTO {
    #[validate(
        required(message = "需要输入邮箱"),
        regex(path = "common::utils::validators::EMAIL_REGEX", message = "邮箱格式不正确")
    )]
    pub email: Option<String>,
    #[validate(
        required(message = "需要输入验证码"),
        length(min = 6, max = 6, message = "验证码长度必须为6位")
    )]
    pub verification_code: Option<String>,
}
