use serde::{Deserialize, Serialize};
use validator::Validate;

///请求获取朋友
#[derive(Clone, Deserialize, Serialize, Debug, Validate)]
pub struct FriendDTO {
    pub username: Option<String>,
    pub account: Option<String>,
}