use serde::{Deserialize, Serialize};
use validator::Validate;

/// 发送二次认证因素验证码(当前仅支持 email)
#[derive(Clone, Debug, Serialize, Deserialize, Validate)]
pub struct SendAuthFactorCodeDTO {
    /// 因素类型: 0=email 1=phone 2=other
    pub factor_type: i16,
    /// 场景: 缺省/`bind`=绑定发码(发往 `factor_value`); `unbind`=解绑发码(发往当前已绑定邮箱)
    #[serde(default)]
    pub scene: Option<String>,
    /// 因素值(邮箱/手机号/其他标识); scene=bind 时必填
    pub factor_value: Option<String>,
}

/// 新增二次认证因素(email 需邮箱验证码)
#[derive(Clone, Debug, Serialize, Deserialize, Validate)]
pub struct CreateAuthFactorDTO {
    /// 因素类型: 0=email 1=phone 2=other
    pub factor_type: i16,
    /// 因素值(邮箱/手机号/其他标识)
    #[validate(length(min = 1, max = 255, message = "因素值长度必须在1-255之间"))]
    pub factor_value: String,
    /// 邮箱绑定验证码(email 类型必填)
    pub verification_code: Option<String>,
}

/// 更新二次认证因素(按 id, 仅本人)
#[derive(Clone, Debug, Serialize, Deserialize, Validate)]
pub struct UpdateAuthFactorDTO {
    pub id: i64,
    #[validate(length(min = 1, max = 255, message = "因素值长度必须在1-255之间"))]
    pub factor_value: Option<String>,
    pub enabled: Option<bool>,
    pub is_primary: Option<bool>,
    pub status: Option<i16>,
}

/// 按 id 查询二次认证因素
#[derive(Clone, Debug, Serialize, Deserialize, Validate)]
pub struct AuthFactorIdDTO {
    pub id: i64,
}

/// 删除(解绑)二次认证因素(email 需解绑验证码)
#[derive(Clone, Debug, Serialize, Deserialize, Validate)]
pub struct DeleteAuthFactorDTO {
    pub id: i64,
    /// 因素类型: 0=email 1=phone 2=other
    pub factor_type: i16,
    /// 解绑验证码(email 类型必填)
    pub verification_code: Option<String>,
}
