use serde::{Deserialize, Serialize};
use validator::Validate;

///请求获取朋友
#[derive(Clone, Deserialize, Serialize, Debug, Validate)]
pub struct Friend {
    pub username: Option<String>,
    #[validate(required(message = "需要输入名称"), length(min = 5, message = "账号长度必须大于5"))]
    pub account: Option<String>
}