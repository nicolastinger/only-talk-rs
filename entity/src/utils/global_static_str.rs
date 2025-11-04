// redis分隔符
pub static REDIS_SPLIT: &str = ":";
pub static REDIS_QUIC_SERVERS: &str = "QUIC:SERVER:";
// 服务
pub static SYSTEM: &str = "system";
// ping/pong
pub static PING: &str = "ping";
pub static PONG: &str = "pong";
// 服务名
pub static SERVER_NAME: &str = "SERVER_1";
// 最大quic连接
pub static MAX_QUIC_SERVERS: usize = 1000;
//最大缓存长度
pub static MAX_QUIC_BUFFER_LEN: usize = 1024 * 1024 * 10;
// 用户已读消息，用户id:对方id，nanoid
pub static USER_READ_MSG: &str = "USER:READ:MSG:";
// 用户发起好友申请
pub static USER_ADD_FRIEND: &str = "USER_ADD_FRIEND_REQUEST";
// 用户处理好友申请
pub static USER_PROCESS_FRIEND: &str = "USER_PROCESS_FRIEND_REQUEST";
