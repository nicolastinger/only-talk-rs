use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct BasePageDTO {
    pub page_num: Option<u32>,
    pub page_size: Option<u32>,
    pub total: Option<u32>,
    /// 任务08: 单聊历史直达会话标识(可选); 缺省时由 path 的 friend_uuid 派生(兼容旧客户端)
    #[serde(default)]
    pub session_uuid: Option<String>,
}
