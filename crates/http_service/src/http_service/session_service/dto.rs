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

// ===== 任务05: 离线同步 /session/sync =====

/// 同步请求
#[derive(Debug, Deserialize, Serialize)]
pub struct SyncRequestDTO {
    /// "incremental"(默认) | "initial"; 非法值按 incremental 处理
    pub mode: Option<String>,
    /// 指定会话(可空); 元素为 session_uuid 字符串
    pub sessions: Option<Vec<String>>,
    /// 单会话上限, 服务端 clamp [1, SYNC_MAX_LIMIT]
    pub limit: Option<u32>,
}

/// 同步消息 VO(单/群同形态, 字段名与 `TextQuicMsg` 对齐, 供客户端直接入库)
#[derive(Debug, Deserialize, Serialize)]
pub struct SyncMessageVO {
    pub id: i64,
    pub nano_id: String,
    pub session_uuid: String,
    /// 1-单聊 2-群聊: 客户端分流依据(与 text_type 值域解释联动)
    pub session_type: i16,
    pub send_user: String,
    /// 单聊: 接收方; 群聊: 即 group_uuid
    pub recv_user: String,
    /// 单聊: text_type; 群聊: msg_type
    pub text_type: u16,
    pub timestamp: i64,
    /// 消息内容(与现有接口同形态: 字节数组)
    pub raw: Vec<u8>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SyncSessionVO {
    pub session_uuid: String,
    pub session_type: i16,
    /// 会话内按 id 升序
    pub messages: Vec<SyncMessageVO>,
    /// 本批最大消息 id; 客户端落库成功后经 /session/synced 回报
    pub next_cursor: i64,
    pub has_more: bool,
    pub truncated_by_window: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SyncResponseVO {
    pub server_time: i64,
    pub sessions: Vec<SyncSessionVO>,
}
