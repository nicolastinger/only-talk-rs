// redis分隔符
pub static REDIS_SPLIT: &str = ":";
pub static REDIS_QUIC_SERVERS: &str = "QUIC:SERVER:";
pub static REDIS_EXTERNAL_QUIC_SERVERS: &str = "QUIC:SERVER:EXTERNAL:";
pub static REDIS_INTERNAL_QUIC_SERVERS: &str = "INTERNAL:QUIC:SERVER:";
// 服务
pub static SYSTEM: &str = "system";
// ping/pong
pub static PING: &str = "ping";
pub static PONG: &str = "pong";
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
// pc平台
pub static PC_PLATFORM: &str = "PC";
// 移动平台
pub static MOBILE_PLATFORM: &str = "MOBILE";
// S3 OSS类型（对应FileUploadRecord.oss_type字段）
pub static OSS_TYPE_MINIO: i32 = 0;
pub static OSS_TYPE_ALIYUN: i32 = 1;
pub static OSS_TYPE_AWS: i32 = 2;
pub static OSS_TYPE_OTHER: i32 = 3;
// S3存储桶默认名称
pub static S3_DEFAULT_BUCKET: &str = "only-talk-rs";
// S3聊天文件预览桶名称（压缩文件）
pub static S3_CHAT_FILE_PREVIEW_BUCKET: &str = "chat-file-preview";
// S3聊天文件原文件桶名称
pub static S3_CHAT_FILE_ORIGIN_BUCKET: &str = "chat-file-origin";
// S3用户头像桶名称
pub static S3_USER_AVATAR_BUCKET: &str = "user-avatar";
// S3 provider名称
pub static S3_PROVIDER_MINIO: &str = "minio";
pub static S3_PROVIDER_ALIYUN_OSS: &str = "aliyun_oss";
pub static S3_PROVIDER_AWS_S3: &str = "aws_s3";
