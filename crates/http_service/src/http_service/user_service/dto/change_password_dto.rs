use serde::{Deserialize, Serialize};
use validator::Validate;

/// 修改密码: 发送验证码请求(由客户端决定认证渠道 factor_type)
#[derive(Clone, Deserialize, Serialize, Debug, Validate)]
pub struct ChangePasswordSendCodeDTO {
    /// 认证因素类型: 0=email 1=phone 2=other
    pub factor_type: i16,
}

/// 修改密码: 校验验证码并写入新密码
#[derive(Clone, Deserialize, Serialize, Debug, Validate)]
pub struct ChangePasswordDTO {
    /// 认证因素类型: 0=email 1=phone 2=other
    pub factor_type: i16,
    #[validate(
        required(message = "需要输入验证码"),
        length(min = 6, max = 6, message = "验证码长度必须为6位")
    )]
    pub verification_code: Option<String>,
    #[validate(
        required(message = "需要输入新密码"),
        regex(
            path = "common::utils::validators::PASSWORD_REGEX",
            message = "密码必须为14位以上的字母或数字"
        )
    )]
    pub new_password: Option<String>,
}
