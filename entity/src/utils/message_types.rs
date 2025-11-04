//! 全局消息类型定义

/// 普通文本消息
pub const MSG_TYPE_TEXT: u16 = 1;

/// 图片消息
pub const MSG_TYPE_IMAGE: u16 = 2;

/// 文件消息
pub const MSG_TYPE_FILE: u16 = 3;

/// P2P消息
pub const MSG_TYPE_P2P: u16 = 4;

/// P2P视频呼叫
pub const MSG_TYPE_P2P_VIDEO_CALL: u16 = 5;

/// P2P视频数据
pub const MSG_TYPE_P2P_VIDEO_DATA: u16 = 6;

/// P2P视频配置
pub const MSG_TYPE_P2P_VIDEO_CONFIG: u16 = 7;

/// 心跳消息(Ping)
pub const MSG_TYPE_PING: u16 = 99;

/// 消息接收成功回执
pub const MSG_TYPE_RECALL_SUCCESS: u16 = 201;

/// 消息接收失败回执
pub const MSG_TYPE_RECALL_FAILURE: u16 = 202;

/// P2P服务端发起
pub const MSG_TYPE_P2P_USER_SERVER: u16 = 203;

/// P2P客户端
pub const MSG_TYPE_P2P_USER_CLIENT: u16 = 204;

/// 系统通知消息起始值
pub const MSG_TYPE_SYSTEM_START: u16 = 10001;
