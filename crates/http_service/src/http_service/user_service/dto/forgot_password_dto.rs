use serde::{Deserialize, Serialize};
use validator::Validate;

/// 忘记密码(未登录)第一步: 向用户已绑定的认证因素发送找回验证码。
///
/// 同一邮箱可能被多个账号绑定, 故需同时提交账号(唯一标识用户);
/// 服务端按账号定位用户, 并校验其确实绑定了所提交的因素值后再发码。
#[derive(Clone, Deserialize, Serialize, Debug, Validate)]
pub struct ForgotPasswordSendCodeDTO {
    /// 登录账号
    #[validate(required(message = "需要输入账号"), length(min = 5, message = "账号长度必须大于5"))]
    pub account: Option<String>,
    /// 认证因素类型: 0=email 1=phone 2=other(当前仅 email)
    pub factor_type: i16,
    /// 因素值(已绑定邮箱/手机号)
    #[validate(required(message = "需要输入邮箱或手机号"))]
    pub factor_value: Option<String>,
}

/// 忘记密码(未登录)第二步: 校验验证码并重置密码。
#[derive(Clone, Deserialize, Serialize, Debug, Validate)]
pub struct ForgotPasswordResetDTO {
    /// 登录账号
    #[validate(required(message = "需要输入账号"), length(min = 5, message = "账号长度必须大于5"))]
    pub account: Option<String>,
    /// 认证因素类型: 0=email 1=phone 2=other(当前仅 email)
    pub factor_type: i16,
    /// 因素值(已绑定邮箱/手机号), 需与发码时一致
    #[validate(required(message = "需要输入邮箱或手机号"))]
    pub factor_value: Option<String>,
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
