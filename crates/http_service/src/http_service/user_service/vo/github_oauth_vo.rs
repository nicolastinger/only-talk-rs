use serde::{Deserialize, Serialize};

/// GitHub OAuth 授权地址响应
#[derive(Debug, Serialize, Deserialize)]
pub struct GithubAuthorizeVO {
    /// 用户应打开的 GitHub 授权地址（系统浏览器）
    pub authorize_url: String,
    /// CSRF state，回调校验用
    pub state: String,
}
