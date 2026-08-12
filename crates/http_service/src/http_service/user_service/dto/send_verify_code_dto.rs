use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Clone, Deserialize, Serialize, Debug, Validate)]
pub struct SendVerifyCodeDTO {
    #[validate(
        required(message = "需要输入邮箱"),
        regex(path = "common::utils::validators::EMAIL_REGEX", message = "邮箱格式不正确")
    )]
    pub email: Option<String>,
}
