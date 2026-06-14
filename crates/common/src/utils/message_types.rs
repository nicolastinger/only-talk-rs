//! Global message type definitions

/// Plain text message
pub const MSG_TYPE_TEXT: u16 = 1;

/// Image message
pub const MSG_TYPE_IMAGE: u16 = 2;

/// File message
pub const MSG_TYPE_FILE: u16 = 3;

/// P2P 消息（客户端直连通信，服务端仅转发）
pub const MSG_TYPE_P2P: u16 = 4;

/// P2P 视频呼叫（客户端直连，服务端仅转发）
pub const MSG_TYPE_P2P_VIDEO_CALL: u16 = 5;

/// P2P 视频数据（客户端直连，服务端仅转发）
pub const MSG_TYPE_P2P_VIDEO_DATA: u16 = 6;

/// P2P 视频配置（客户端直连，服务端仅转发）
pub const MSG_TYPE_P2P_VIDEO_CONFIG: u16 = 7;

/// Heartbeat message (Ping)
pub const MSG_TYPE_PING: u16 = 99;

/// Message delivery success receipt
pub const MSG_TYPE_RECALL_SUCCESS: u16 = 201;

/// Message delivery failure receipt
pub const MSG_TYPE_RECALL_FAILURE: u16 = 202;

/// 通知客户端作为 P2P 连接的服务端（NAT 发现后由服务端下发）
pub const MSG_TYPE_P2P_USER_SERVER: u16 = 203;

/// 通知客户端作为 P2P 连接的客户端（NAT 发现后由服务端下发）
pub const MSG_TYPE_P2P_USER_CLIENT: u16 = 204;

/// Notification message
pub const NOTIFY_TYPE_MSG: u16 = 1024;

/// System message
pub const MSG_TYPE_SYSTEM: u16 = 10001;

/// Friend notification message forwarded by internal service
pub const INTERNAL_FRIEND_NOTIFY: u16 = 20001;

// ==================== Group Chat Message Types ====================

/// Group text message
pub const MSG_TYPE_GROUP_TEXT: u16 = 2001;

/// Group image message
pub const MSG_TYPE_GROUP_IMAGE: u16 = 2002;

/// Group file message
pub const MSG_TYPE_GROUP_FILE: u16 = 2003;

/// Group notification message (member changes, etc.)
pub const MSG_TYPE_GROUP_NOTIFICATION: u16 = 2004;

/// Group message delivery success receipt
pub const MSG_TYPE_GROUP_ACK: u16 = 2201;
