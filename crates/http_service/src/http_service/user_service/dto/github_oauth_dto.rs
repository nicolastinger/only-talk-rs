use serde::{Deserialize, Serialize};
use validator::Validate;

/// GitHub OAuth 第一步: 请求授权地址（返回 authorize_url + state）
#[derive(Debug, Validate, Serialize, Deserialize)]
pub struct GithubAuthorizeDTO {
    #[validate(required(message = "需要输入平台"), length(min = 2, message = "平台长度必须大于5"))]
    pub platform: Option<String>,
    #[validate(
        required(message = "需要输入设备指纹"),
        length(min = 16, message = "设备指纹长度必须大于16")
    )]
    pub device_fingerprint: Option<String>,
}

/// GitHub OAuth 第二步: 回环回调拿到 code 后换取登录态
#[derive(Debug, Validate, Serialize, Deserialize)]
pub struct GithubOAuthCallbackDTO {
    /// GitHub 授权码（一次性）
    #[validate(required(message = "需要输入授权码"), length(min = 1, message = "授权码不能为空"))]
    pub code: Option<String>,
    /// CSRF state，须与发起时的 state 一致（单次消费）
    #[validate(required(message = "需要输入state"), length(min = 1, message = "state不能为空"))]
    pub state: Option<String>,
    #[validate(required(message = "需要输入平台"), length(min = 2, message = "平台长度必须大于5"))]
    pub platform: Option<String>,
    #[validate(
        required(message = "需要输入设备指纹"),
        length(min = 16, message = "设备指纹长度必须大于16")
    )]
    pub device_fingerprint: Option<String>,
}
