use serde::{Deserialize, Serialize};

/// 两步注册第一步响应: 注册会话 token + 占位用户 uuid
#[derive(Debug, Serialize, Deserialize)]
pub struct SignUpStep1ResponseVO {
    /// 注册会话 token(第二步补全资料时使用, 短时效)
    pub reg_token: String,
    /// 占位用户 uuid
    pub uuid: String,
}
