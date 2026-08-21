// Redis 键分隔符
pub static REDIS_SPLIT: &str = ":";
pub static REDIS_QUIC_SERVERS: &str = "QUIC:SERVER:";
pub static REDIS_EXTERNAL_QUIC_SERVERS: &str = "QUIC:SERVER:EXTERNAL:";
pub static REDIS_INTERNAL_QUIC_SERVERS: &str = "INTERNAL:QUIC:SERVER:";
// 服务
pub static SYSTEM: &str = "system";
// ping/pong 心跳消息
pub static PING: &str = "ping";
pub static PONG: &str = "pong";
// QUIC 最大连接数
pub static MAX_QUIC_SERVERS: usize = 1000;
// 最大缓冲区长度
pub static MAX_QUIC_BUFFER_LEN: usize = 1024 * 1024 * 10;
// 用户已读消息: user_id:other_id, nanoid
pub static USER_READ_MSG: &str = "USER:READ:MSG:";
// 用户发起好友请求
pub static USER_ADD_FRIEND: &str = "USER_ADD_FRIEND_REQUEST";
// 用户处理好友请求
pub static USER_PROCESS_FRIEND: &str = "USER_PROCESS_FRIEND_REQUEST";
// NAT UDP 用户地址信息
pub static USER_UDP_ADDRESS: &str = "USER_UDP_ADDRESS_";
// NAT UDP 用户地址锁
pub static USER_UDP_ADDRESS_LOCK: &str = "USER_UDP_ADDRESS_LOCK_";
// 群成员列表缓存
pub static GROUP_MEMBERS_CACHE: &str = "GROUP:MEMBERS:";
// 刷新令牌映射
pub static REFRESH_TOKEN: &str = "REFRESH_TOKEN:";
// 刷新令牌平台映射
pub static REFRESH_TOKEN_PLATFORM: &str = "REFRESH_TOKEN:PLATFORM:";
// 邮箱注册验证码
pub static EMAIL_VERIFY_CODE: &str = "EMAIL:VERIFY:CODE:";// 默认最大文件大小 (20MB)
pub static DEFAULT_MAX_FILE_SIZE: i64 = 20 * 1024 * 1024;
// 注册会话 token(两步注册,step1 验证通过后下发,映射占位用户 uuid)
pub static REGISTER_SESSION_TOKEN: &str = "REGISTER:SESSION:TOKEN:";
// PC 平台
pub static PC_PLATFORM: &str = "PC";
// 移动端平台
pub static MOBILE_PLATFORM: &str = "MOBILE";
// S3 OSS 类型(对应 FileUploadRecord.oss_type 字段)
pub static OSS_TYPE_MINIO: i32 = 0;
pub static OSS_TYPE_ALIYUN: i32 = 1;
pub static OSS_TYPE_AWS: i32 = 2;
pub static OSS_TYPE_OTHER: i32 = 3;
// S3 存储桶默认名称
pub static S3_DEFAULT_BUCKET: &str = "only-talk-rs";
// S3 聊天文件预览桶名称(压缩文件)
pub static S3_CHAT_FILE_PREVIEW_BUCKET: &str = "chat-file-preview";
// S3 聊天文件原文件桶名称
pub static S3_CHAT_FILE_ORIGIN_BUCKET: &str = "chat-file-origin";
// S3 用户头像桶名称
pub static S3_USER_AVATAR_BUCKET: &str = "user-avatar";
// S3 提供商名称
pub static S3_PROVIDER_MINIO: &str = "minio";
pub static S3_PROVIDER_ALIYUN_OSS: &str = "aliyun_oss";
pub static S3_PROVIDER_AWS_S3: &str = "aws_s3";
