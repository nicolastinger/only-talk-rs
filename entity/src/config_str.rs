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
// 用户上传文件公开目录
pub static USER_FILE_PUBLIC_DIR: &str = "./resources/pub_file/";
// 用户上传文件公开路径
pub static USER_FILE_PUBLIC: &str = "/resources";
// 默认用户头像
pub static USER_DEFAULT_ICON: &str = "73983c6e-2f52-4fe5-95e8-f4302abc223d.jpg";
// 应用域名
pub static APP_DOMAIN: &str = "https://onlytalk.cn:8443";
// 默认最大文件大小 (20MB)
pub static DEFAULT_MAX_FILE_SIZE: i64 = 20 * 1024 * 1024;
