use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SignInResponseVO {
    /// 短效 access token
    pub access_token: String,
    /// 长效 refresh token
    pub refresh_token: String,
}
