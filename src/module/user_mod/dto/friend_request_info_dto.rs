use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FriendRequestInfoDTO {
    // 请求信息
    pub request_message: Option<String>,
    pub accept_message: Option<String>,
    pub request_user: Option<String>,
    pub accept_user: Option<String>,
    // 添加方式
    pub add_type: Option<String>,
    pub version: Option<u32>,
    // 状态
    pub accept_status: Option<u8>,
}