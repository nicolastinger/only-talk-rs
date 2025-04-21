use serde::{Deserialize, Serialize};
use validator::Validate;

///请求获取朋友
#[derive(Clone, Deserialize, Serialize, Debug, Validate)]
pub struct FriendDTO {
    pub username: Option<String>,
    pub account: Option<String>,
}


#[derive(Clone, Deserialize, Serialize, Debug, Validate)]
pub struct FriendLinkDTO {
    #[validate(required(message = "需要输入id"), length(min = 5, message = "账号长度必须大于5"))]
    pub account: Option<String>,
    pub enable: Option<bool>
}