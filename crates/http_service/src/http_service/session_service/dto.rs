use serde::{Deserialize, Serialize};

/// 已读上报(新契约): 客户端直接报会话游标
#[derive(Debug, Deserialize, Serialize)]
pub struct SessionReadItem {
    pub session_uuid: String,
    /// 1-单聊 2-群聊(决定游标值域, 见主方案 §4.4)
    pub session_type: i16,
    pub last_read_id: i64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SessionReadDTO {
    pub reads: Vec<SessionReadItem>,
}

/// 同步游标回报(§7.3): 客户端落库成功后推进
#[derive(Debug, Deserialize, Serialize)]
pub struct SessionSyncedItem {
    pub session_uuid: String,
    pub synced_id: i64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SessionSyncedDTO {
    pub sessions: Vec<SessionSyncedItem>,
}
