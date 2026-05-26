//! 全局消息类型定义

/// 普通文本消息
pub const MSG_TYPE_TEXT: u16 = 1;

/// 图片消息
pub const MSG_TYPE_IMAGE: u16 = 2;

/// 文件消息
pub const MSG_TYPE_FILE: u16 = 3;

/// P2P 消息（客户端直连通信，服务端仅转发）
pub const MSG_TYPE_P2P: u16 = 4;

/// P2P 视频呼叫（客户端直连，服务端仅转发）
pub const MSG_TYPE_P2P_VIDEO_CALL: u16 = 5;

/// P2P 视频数据（客户端直连，服务端仅转发）
pub const MSG_TYPE_P2P_VIDEO_DATA: u16 = 6;

/// P2P 视频配置（客户端直连，服务端仅转发）
pub const MSG_TYPE_P2P_VIDEO_CONFIG: u16 = 7;

/// 心跳消息(Ping)
pub const MSG_TYPE_PING: u16 = 99;

/// 消息接收成功回执
pub const MSG_TYPE_RECALL_SUCCESS: u16 = 201;

/// 消息接收失败回执
pub const MSG_TYPE_RECALL_FAILURE: u16 = 202;

/// 通知客户端作为 P2P 连接的服务端（NAT 发现后由服务端下发）
pub const MSG_TYPE_P2P_USER_SERVER: u16 = 203;

/// 通知客户端作为 P2P 连接的客户端（NAT 发现后由服务端下发）
pub const MSG_TYPE_P2P_USER_CLIENT: u16 = 204;

/// 通知消息
pub const NOTIFY_TYPE_MSG: u16 = 1024;

/// 系统消息
pub const MSG_TYPE_SYSTEM: u16 = 10001;

/// 内网服务转发的好友通知消息
pub const INTERNAL_FRIEND_NOTIFY: u16 = 20001;


// ==================== 群聊消息类型 ====================

/// 群文本消息
pub const MSG_TYPE_GROUP_TEXT: u16 = 2001;

/// 群图片消息
pub const MSG_TYPE_GROUP_IMAGE: u16 = 2002;

/// 群文件消息
pub const MSG_TYPE_GROUP_FILE: u16 = 2003;

/// 群通知消息（成员变更等）
pub const MSG_TYPE_GROUP_NOTIFICATION: u16 = 2004;
