//! common 无外部依赖(数据库/Redis/网络)函数的单元测试

use std::sync::LazyLock;
use std::sync::atomic::Ordering;

use common::config_manager::set_config;
use common::config_str::{
    DEFAULT_MAX_FILE_SIZE, EMAIL_VERIFY_CODE, GROUP_MEMBERS_CACHE, MAX_QUIC_BUFFER_LEN,
    MAX_QUIC_SERVERS, MOBILE_PLATFORM, OSS_TYPE_ALIYUN, OSS_TYPE_AWS, OSS_TYPE_MINIO,
    OSS_TYPE_OTHER, PC_PLATFORM, PING, PONG, REDIS_EXTERNAL_QUIC_SERVERS,
    REDIS_INTERNAL_QUIC_SERVERS, REDIS_QUIC_SERVERS, REDIS_SPLIT, REFRESH_TOKEN,
    S3_CHAT_FILE_ORIGIN_BUCKET, S3_CHAT_FILE_PREVIEW_BUCKET, S3_DEFAULT_BUCKET,
    S3_PROVIDER_ALIYUN_OSS, S3_PROVIDER_AWS_S3, S3_PROVIDER_MINIO, S3_USER_AVATAR_BUCKET, SYSTEM,
    USER_ADD_FRIEND, USER_PROCESS_FRIEND, USER_READ_MSG, USER_UDP_ADDRESS, USER_UDP_ADDRESS_LOCK,
};
use common::substitute_env_vars;
use common::utils::group_msg::{
    BroadcastType, GroupQuicMsg, InternalGroupBroadcast, InternalGroupBroadcastResponse,
};
use common::utils::internal_quic_client::make_internal_client_config;
use common::utils::internal_quic_msg::{InternalQuicRequest, InternalQuicResponse, RequestSource};
use common::utils::jwt_util::{generate_access_token, generate_token_with_expiry, verify_token};
use common::utils::message_types::{
    INTERNAL_FRIEND_NOTIFY, MSG_TYPE_FILE, MSG_TYPE_GROUP_ACK, MSG_TYPE_GROUP_FILE,
    MSG_TYPE_GROUP_IMAGE, MSG_TYPE_GROUP_NOTIFICATION, MSG_TYPE_GROUP_TEXT, MSG_TYPE_IMAGE,
    MSG_TYPE_P2P, MSG_TYPE_P2P_USER_CLIENT, MSG_TYPE_P2P_USER_SERVER, MSG_TYPE_P2P_VIDEO_CALL,
    MSG_TYPE_P2P_VIDEO_CONFIG, MSG_TYPE_P2P_VIDEO_DATA, MSG_TYPE_PING, MSG_TYPE_RECALL_FAILURE,
    MSG_TYPE_RECALL_SUCCESS, MSG_TYPE_SYSTEM, MSG_TYPE_TEXT, NOTIFY_TYPE_MSG,
};
use common::utils::rsa_util::{
    generate_random_string, get_rsa_keys, hash_password, verify_password,
};
use common::utils::server_count_sync::{SERVER_COUNT, compute_preferred_index, get_server_count};
use common::utils::text_msg::{
    HeadMsg, MessageType, TextMsg, TextQuicMsg, X25, build_text_msg, generate_text_msg_with_id,
    generate_text_msg_with_time,
};
use common::utils::time::{get_now_time_stamp_as_millis, get_now_time_stamp_as_secs};
use common::utils::validators::{EMAIL_REGEX, PASSWORD_REGEX};
use rsa::pkcs1::EncodeRsaPublicKey;
use rsa::pkcs8::EncodePrivateKey;
use rsa::traits::PublicKeyParts;
use rsa::{RsaPrivateKey, RsaPublicKey};

/// 预生成一套测试用 RSA 密钥(2048 位),供 JWT / get_rsa_keys 测试通过
/// config_manager 内存缓存路径复用,避免测试期间反复生成与写文件。
static TEST_KEYS: LazyLock<(String, String)> = LazyLock::new(|| {
    let mut rng = rand::thread_rng();
    let private_key = RsaPrivateKey::new(&mut rng, 2048).expect("生成测试 RSA 私钥失败");
    let public_key = RsaPublicKey::from(&private_key);
    let private_pem = private_key
        .to_pkcs8_pem(Default::default())
        .expect("导出测试 RSA 私钥 PEM 失败")
        .to_string();
    let public_pem = public_key
        .to_pkcs1_pem(Default::default())
        .expect("导出测试 RSA 公钥 PEM 失败")
        .to_string();
    (private_pem, public_pem)
});

/// 把测试密钥写入内存 config,使 get_rsa_keys 走内存缓存路径(不触文件系统)
fn install_test_jwt_keys() {
    set_config("jwt_private_key".to_string(), TEST_KEYS.0.clone());
    set_config("jwt_public_key".to_string(), TEST_KEYS.1.clone());
}

/// lib.rs:substitute_env_vars(环境变量占位符替换,纯字符串逻辑)
mod env_var_substitution {
    use super::*;

    #[test]
    fn replaces_single_placeholder() {
        unsafe {
            std::env::set_var("TEST_COMMON_SUBST", "hello");
        }
        let result = substitute_env_vars("db=${TEST_COMMON_SUBST}://localhost".to_string());
        assert_eq!(result, "db=hello://localhost");
        unsafe {
            std::env::remove_var("TEST_COMMON_SUBST");
        }
    }

    #[test]
    fn replaces_multiple_placeholders() {
        unsafe {
            std::env::set_var("TEST_COMMON_SUBST_A", "first");
            std::env::set_var("TEST_COMMON_SUBST_B", "second");
        }
        let result = substitute_env_vars(
            "${TEST_COMMON_SUBST_A}-${TEST_COMMON_SUBST_B}-${TEST_COMMON_SUBST_A}".to_string(),
        );
        assert_eq!(result, "first-second-first");
        unsafe {
            std::env::remove_var("TEST_COMMON_SUBST_A");
            std::env::remove_var("TEST_COMMON_SUBST_B");
        }
    }

    #[test]
    fn missing_var_becomes_empty() {
        let result = substitute_env_vars("val=${TEST_COMMON_NOT_SET_XYZ}".to_string());
        assert_eq!(result, "val=");
    }

    #[test]
    fn without_placeholder_unchanged() {
        let result = substitute_env_vars("no placeholders here".to_string());
        assert_eq!(result, "no placeholders here");
    }

    #[test]
    fn terminates_on_circular_reference() {
        unsafe {
            std::env::set_var("TEST_COMMON_SUBST_C1", "${TEST_COMMON_SUBST_C2}");
            std::env::set_var("TEST_COMMON_SUBST_C2", "${TEST_COMMON_SUBST_C1}");
        }
        let result = substitute_env_vars("${TEST_COMMON_SUBST_C1}".to_string());
        // 循环引用应在 100 次迭代后被强制终止,而不是死循环
        assert!(result.contains("${"));
        unsafe {
            std::env::remove_var("TEST_COMMON_SUBST_C1");
            std::env::remove_var("TEST_COMMON_SUBST_C2");
        }
    }
}

/// config_str 关键常量
mod config_str {
    use super::*;

    #[test]
    fn key_constants_values() {
        assert_eq!(REDIS_SPLIT, ":");
        assert_eq!(MAX_QUIC_SERVERS, 1000);
        assert_eq!(PC_PLATFORM, "PC");
        assert_eq!(S3_DEFAULT_BUCKET, "only-talk-rs");
    }

    #[test]
    fn redis_key_prefixes() {
        assert_eq!(REDIS_QUIC_SERVERS, "QUIC:SERVER:");
        assert_eq!(REDIS_EXTERNAL_QUIC_SERVERS, "QUIC:SERVER:EXTERNAL:");
        assert_eq!(REDIS_INTERNAL_QUIC_SERVERS, "INTERNAL:QUIC:SERVER:");
        assert_eq!(USER_READ_MSG, "USER:READ:MSG:");
        assert_eq!(USER_ADD_FRIEND, "USER_ADD_FRIEND_REQUEST");
        assert_eq!(USER_PROCESS_FRIEND, "USER_PROCESS_FRIEND_REQUEST");
        assert_eq!(USER_UDP_ADDRESS, "USER_UDP_ADDRESS_");
        assert_eq!(USER_UDP_ADDRESS_LOCK, "USER_UDP_ADDRESS_LOCK_");
        assert_eq!(GROUP_MEMBERS_CACHE, "GROUP:MEMBERS:");
        assert_eq!(REFRESH_TOKEN, "REFRESH_TOKEN:");
        assert_eq!(EMAIL_VERIFY_CODE, "EMAIL:VERIFY:CODE:");
    }

    #[test]
    fn service_and_platform_constants() {
        assert_eq!(SYSTEM, "system");
        assert_eq!(PING, "ping");
        assert_eq!(PONG, "pong");
        assert_eq!(MOBILE_PLATFORM, "MOBILE");
    }

    #[test]
    fn limits_and_s3_constants() {
        assert_eq!(MAX_QUIC_BUFFER_LEN, 1024 * 1024 * 10);
        assert_eq!(DEFAULT_MAX_FILE_SIZE, 20 * 1024 * 1024);
        assert_eq!(OSS_TYPE_MINIO, 0);
        assert_eq!(OSS_TYPE_ALIYUN, 1);
        assert_eq!(OSS_TYPE_AWS, 2);
        assert_eq!(OSS_TYPE_OTHER, 3);
        assert_eq!(S3_CHAT_FILE_PREVIEW_BUCKET, "chat-file-preview");
        assert_eq!(S3_CHAT_FILE_ORIGIN_BUCKET, "chat-file-origin");
        assert_eq!(S3_USER_AVATAR_BUCKET, "user-avatar");
        assert_eq!(S3_PROVIDER_MINIO, "minio");
        assert_eq!(S3_PROVIDER_ALIYUN_OSS, "aliyun_oss");
        assert_eq!(S3_PROVIDER_AWS_S3, "aws_s3");
    }
}

/// utils::message_types 常量
mod message_types {
    use super::*;

    #[test]
    fn constants_values() {
        assert_eq!(MSG_TYPE_TEXT, 1);
        assert_eq!(MSG_TYPE_IMAGE, 2);
        assert_eq!(MSG_TYPE_FILE, 3);
        assert_eq!(MSG_TYPE_P2P, 4);
        assert_eq!(MSG_TYPE_P2P_VIDEO_CALL, 5);
        assert_eq!(MSG_TYPE_P2P_VIDEO_DATA, 6);
        assert_eq!(MSG_TYPE_P2P_VIDEO_CONFIG, 7);
        assert_eq!(MSG_TYPE_PING, 99);
        assert_eq!(MSG_TYPE_RECALL_SUCCESS, 201);
        assert_eq!(MSG_TYPE_RECALL_FAILURE, 202);
        assert_eq!(MSG_TYPE_P2P_USER_SERVER, 203);
        assert_eq!(MSG_TYPE_P2P_USER_CLIENT, 204);
        assert_eq!(NOTIFY_TYPE_MSG, 1024);
        assert_eq!(MSG_TYPE_SYSTEM, 10001);
        assert_eq!(INTERNAL_FRIEND_NOTIFY, 20001);
        assert_eq!(MSG_TYPE_GROUP_TEXT, 2001);
        assert_eq!(MSG_TYPE_GROUP_IMAGE, 2002);
        assert_eq!(MSG_TYPE_GROUP_FILE, 2003);
        assert_eq!(MSG_TYPE_GROUP_NOTIFICATION, 2004);
        assert_eq!(MSG_TYPE_GROUP_ACK, 2201);
    }
}

/// utils::time 时间戳辅助函数
mod time {
    use super::*;

    #[test]
    fn secs_and_millis_are_positive_and_consistent() {
        let secs = get_now_time_stamp_as_secs().expect("获取当前秒级时间戳失败");
        let millis = get_now_time_stamp_as_millis().expect("获取当前毫秒级时间戳失败");
        assert!(secs > 0);
        assert!(millis > 0);
        assert!(millis >= secs * 1000);
        assert!(millis - secs * 1000 < 5_000);
    }
}

/// utils::validators 正则校验
mod validators {
    use super::*;

    #[test]
    fn email_regex_accepts_valid() {
        assert!(EMAIL_REGEX.is_match("user@example.com"));
        assert!(EMAIL_REGEX.is_match("user.name+tag@sub.example.co.uk"));
    }

    #[test]
    fn email_regex_rejects_invalid() {
        assert!(!EMAIL_REGEX.is_match("invalid@"));
        assert!(!EMAIL_REGEX.is_match("@example.com"));
        assert!(!EMAIL_REGEX.is_match("user@.com"));
        assert!(!EMAIL_REGEX.is_match("a b@c.com"));
    }

    #[test]
    fn password_regex_requires_14_plus_alphanumeric() {
        assert!(PASSWORD_REGEX.is_match("abcdefghijklmn"));
        assert!(PASSWORD_REGEX.is_match("AbCdEf12345678"));
        assert!(!PASSWORD_REGEX.is_match("short"));
        assert!(!PASSWORD_REGEX.is_match("abcdefghijklmn!"));
        assert!(!PASSWORD_REGEX.is_match("abc defghijklmn"));
    }
}

/// utils::text_msg 消息打包 / CRC / 序列化
mod text_msg {
    use super::*;

    fn head_size() -> usize {
        bincode::serialize(&HeadMsg { version: 0, crc: 0, body_len: 0, message_type: 0 })
            .expect("序列化 HeadMsg 失败")
            .len()
    }

    #[test]
    fn message_type_variants_match_constants() {
        assert_eq!(MessageType::Text as u16, MSG_TYPE_TEXT);
        assert_eq!(MessageType::Image as u16, MSG_TYPE_IMAGE);
        assert_eq!(MessageType::File as u16, MSG_TYPE_FILE);
        assert_eq!(MessageType::P2P as u16, MSG_TYPE_P2P);
        assert_eq!(MessageType::P2PVideoCall as u16, MSG_TYPE_P2P_VIDEO_CALL);
        assert_eq!(MessageType::P2pVideoData as u16, MSG_TYPE_P2P_VIDEO_DATA);
        assert_eq!(MessageType::P2pVideoConfig as u16, MSG_TYPE_P2P_VIDEO_CONFIG);
        assert_eq!(MessageType::Ping as u16, MSG_TYPE_PING);
        assert_eq!(MessageType::RecallSuccess as u16, MSG_TYPE_RECALL_SUCCESS);
        assert_eq!(MessageType::RecallFailure as u16, MSG_TYPE_RECALL_FAILURE);
        assert_eq!(MessageType::P2pUserServer as u16, MSG_TYPE_P2P_USER_SERVER);
        assert_eq!(MessageType::P2pUserClient as u16, MSG_TYPE_P2P_USER_CLIENT);
        assert_eq!(MessageType::System as u16, MSG_TYPE_SYSTEM);
    }

    #[test]
    fn head_msg_bytes_roundtrip() {
        let head = HeadMsg { version: 1, crc: 0x1234, body_len: 100, message_type: MSG_TYPE_TEXT };
        let bytes = head.get_bytes().expect("序列化 HeadMsg 失败");
        let de: HeadMsg = bincode::deserialize(&bytes).expect("反序列化 HeadMsg 失败");
        assert_eq!(de.version, head.version);
        assert_eq!(de.crc, head.crc);
        assert_eq!(de.body_len, head.body_len);
        assert_eq!(de.message_type, head.message_type);
    }

    #[test]
    fn text_quic_msg_bytes_roundtrip() {
        let msg = TextQuicMsg {
            nano_id: "nano-1".to_string(),
            text_type: MSG_TYPE_TEXT,
            raw: b"hello".to_vec(),
            recv_user: "user-b".to_string(),
            send_user: "user-a".to_string(),
            timestamp: 1_700_000_000_000,
        };
        let bytes = msg.get_bytes().expect("序列化 TextQuicMsg 失败");
        let de: TextQuicMsg = bincode::deserialize(&bytes).expect("反序列化 TextQuicMsg 失败");
        assert_eq!(de.nano_id, msg.nano_id);
        assert_eq!(de.raw, msg.raw);
        assert_eq!(de.recv_user, msg.recv_user);
        assert_eq!(de.send_user, msg.send_user);
        assert_eq!(de.timestamp, msg.timestamp);
    }

    #[test]
    fn build_text_msg_concatenates_header_and_body() {
        let head = HeadMsg { version: 1, crc: 0x1234, body_len: 5, message_type: MSG_TYPE_TEXT };
        let body = TextQuicMsg {
            nano_id: "nano-1".to_string(),
            text_type: MSG_TYPE_TEXT,
            raw: b"hello".to_vec(),
            recv_user: "user-b".to_string(),
            send_user: "user-a".to_string(),
            timestamp: 1_700_000_000_000,
        };
        let bytes = build_text_msg(&head, &body).expect("拼接 header 与 body 失败");
        let size = head_size();
        assert_eq!(bytes.len(), size + bincode::serialize(&body).expect("序列化 body 失败").len());

        let head_de: HeadMsg = bincode::deserialize(&bytes[..size]).expect("反序列化 header 失败");
        assert_eq!(head_de.version, head.version);
        assert_eq!(head_de.crc, head.crc);
        let body_de: TextQuicMsg =
            bincode::deserialize(&bytes[size..]).expect("反序列化 body 失败");
        assert_eq!(body_de.nano_id, "nano-1");
    }

    #[test]
    fn generate_text_msg_with_time_is_deterministic_and_crc_valid() {
        let bytes = generate_text_msg_with_time(
            "nano-1".to_string(),
            MSG_TYPE_TEXT,
            b"hello".to_vec(),
            "user-b".to_string(),
            "user-a".to_string(),
            1_700_000_000_000,
        )
        .expect("生成文本消息失败");

        let size = head_size();
        let head: HeadMsg = bincode::deserialize(&bytes[..size]).expect("反序列化 header 失败");
        let body: TextQuicMsg = bincode::deserialize(&bytes[size..]).expect("反序列化 body 失败");

        assert_eq!(head.version, 1);
        assert_eq!(head.message_type, MSG_TYPE_TEXT);
        assert_eq!(head.body_len as usize, bytes.len() - size);
        assert_eq!(head.crc, X25.checksum(&bytes[size..]));

        assert_eq!(body.nano_id, "nano-1");
        assert_eq!(body.text_type, MSG_TYPE_TEXT);
        assert_eq!(body.raw, b"hello");
        assert_eq!(body.recv_user, "user-b");
        assert_eq!(body.send_user, "user-a");
        assert_eq!(body.timestamp, 1_700_000_000_000);
    }

    #[test]
    fn generate_text_msg_with_id_uses_current_time() {
        let before = get_now_time_stamp_as_millis().expect("获取当前毫秒级时间戳失败");
        let bytes = generate_text_msg_with_id(
            "nano-2".to_string(),
            MSG_TYPE_TEXT,
            b"hi".to_vec(),
            "user-b".to_string(),
            "user-a".to_string(),
        )
        .expect("生成文本消息失败");
        let after = get_now_time_stamp_as_millis().expect("获取当前毫秒级时间戳失败");

        let size = head_size();
        let body: TextQuicMsg = bincode::deserialize(&bytes[size..]).expect("反序列化 body 失败");
        assert_eq!(body.nano_id, "nano-2");
        assert!(body.timestamp >= before && body.timestamp <= after);
    }
}

/// utils::group_msg 群广播消息类型映射与序列化
mod group_msg {
    use super::*;

    #[test]
    fn broadcast_type_from_msg_type() {
        assert_eq!(BroadcastType::from_msg_type(10), BroadcastType::GroupText);
        assert_eq!(BroadcastType::from_msg_type(11), BroadcastType::GroupImage);
        assert_eq!(BroadcastType::from_msg_type(12), BroadcastType::GroupFile);
        assert_eq!(BroadcastType::from_msg_type(13), BroadcastType::GroupNotification);
        assert_eq!(BroadcastType::from_msg_type(99), BroadcastType::GroupNotification);
    }

    #[test]
    fn broadcast_type_to_msg_type() {
        assert_eq!(BroadcastType::GroupText.to_msg_type(), 10);
        assert_eq!(BroadcastType::GroupImage.to_msg_type(), 11);
        assert_eq!(BroadcastType::GroupFile.to_msg_type(), 12);
        assert_eq!(BroadcastType::GroupNotification.to_msg_type(), 13);
    }

    #[test]
    fn broadcast_type_roundtrip_for_known_types() {
        for msg_type in [10u16, 11, 12, 13] {
            let bt = BroadcastType::from_msg_type(msg_type);
            assert_eq!(BroadcastType::from_msg_type(bt.to_msg_type()), bt);
        }
    }

    #[test]
    fn internal_group_broadcast_serde_roundtrip() {
        let msg = InternalGroupBroadcast {
            broadcast_type: BroadcastType::GroupText,
            group_uuid: "group-1".to_string(),
            msg_bytes: vec![1, 2, 3],
            sender: "user-a".to_string(),
            all_members: vec!["user-a".to_string(), "user-b".to_string()],
            source_node: 0,
            timestamp: 1_700_000_000_000,
            broadcast_id: "bcast-1".to_string(),
        };
        let json = serde_json::to_value(&msg).expect("序列化失败");
        let de: InternalGroupBroadcast =
            serde_json::from_value(json.clone()).expect("反序列化失败");
        assert_eq!(serde_json::to_value(&de).expect("序列化失败"), json);
    }

    #[test]
    fn group_quic_msg_serde_roundtrip() {
        let msg = GroupQuicMsg {
            nano_id: "nano-1".to_string(),
            msg_type: MSG_TYPE_GROUP_TEXT,
            group_uuid: "group-1".to_string(),
            send_user: "user-a".to_string(),
            raw: b"hello".to_vec(),
            timestamp: 1_700_000_000_000,
        };
        let json = serde_json::to_value(&msg).expect("序列化失败");
        let de: GroupQuicMsg = serde_json::from_value(json.clone()).expect("反序列化失败");
        assert_eq!(serde_json::to_value(&de).expect("序列化失败"), json);
    }

    #[test]
    fn group_broadcast_response_constructors() {
        let ok = InternalGroupBroadcastResponse::ok();
        assert_eq!(ok.status, "ok");
        assert!(ok.message.is_none());

        let err = InternalGroupBroadcastResponse::error("boom");
        assert_eq!(err.status, "error");
        assert_eq!(err.message.as_deref(), Some("boom"));
    }
}

/// utils::internal_quic_msg 内部请求/响应结构与序列化
mod internal_quic_msg {
    use super::*;

    fn sample_request() -> InternalQuicRequest {
        InternalQuicRequest {
            msg_type: MSG_TYPE_TEXT,
            payload: b"hello".to_vec(),
            target_user: "user-b".to_string(),
            preferred_index: 1,
            platform: PC_PLATFORM.to_string(),
            source: RequestSource::QuicExternal,
            ttl: 3,
        }
    }

    #[test]
    fn request_serde_roundtrip() {
        let req = sample_request();
        let json = serde_json::to_value(&req).expect("序列化失败");
        let de: InternalQuicRequest = serde_json::from_value(json.clone()).expect("反序列化失败");
        assert_eq!(serde_json::to_value(&de).expect("序列化失败"), json);
    }

    #[test]
    fn request_bincode_roundtrip() {
        let req = sample_request();
        let bytes = bincode::serialize(&req).expect("bincode 序列化失败");
        let de: InternalQuicRequest = bincode::deserialize(&bytes).expect("bincode 反序列化失败");
        assert_eq!(de.msg_type, req.msg_type);
        assert_eq!(de.payload, req.payload);
        assert_eq!(de.target_user, req.target_user);
        assert_eq!(de.preferred_index, req.preferred_index);
        assert_eq!(de.platform, req.platform);
        assert_eq!(de.ttl, req.ttl);
    }

    #[test]
    fn response_serde_roundtrip() {
        for resp in [
            InternalQuicResponse::ok(),
            InternalQuicResponse::error("boom"),
            InternalQuicResponse::user_offline(),
        ] {
            let json = serde_json::to_value(&resp).expect("序列化失败");
            let de: InternalQuicResponse =
                serde_json::from_value(json.clone()).expect("反序列化失败");
            assert_eq!(serde_json::to_value(&de).expect("序列化失败"), json);
        }
    }

    #[test]
    fn response_constructors() {
        let ok = InternalQuicResponse::ok();
        assert_eq!(ok.status, "ok");
        assert_eq!(ok.delivered, Some(true));
        assert!(ok.message.is_none());

        let err = InternalQuicResponse::error("boom");
        assert_eq!(err.status, "error");
        assert_eq!(err.message.as_deref(), Some("boom"));
        assert_eq!(err.delivered, None);

        let offline = InternalQuicResponse::user_offline();
        assert_eq!(offline.status, "ok");
        assert_eq!(offline.delivered, Some(false));
        assert_eq!(offline.message.as_deref(), Some("User offline"));
    }
}

/// utils::rsa_util RSA 密钥 / 密码哈希 / 随机字符串
mod rsa_util {
    use super::*;

    #[test]
    fn get_rsa_keys_returns_2048_bit_pair_from_config() {
        install_test_jwt_keys();
        let (private_key, public_key) = get_rsa_keys().expect("获取 RSA 密钥对失败");
        assert_eq!(private_key.n().bits(), 2048);
        assert_eq!(public_key.n().bits(), 2048);
    }

    #[test]
    fn generate_random_string_length_and_charset() {
        let s = generate_random_string(16);
        assert_eq!(s.len(), 16);
        assert!(s.chars().all(|c| c.is_ascii_alphanumeric()));

        let s2 = generate_random_string(16);
        assert_ne!(s, s2);
    }

    #[test]
    fn hash_and_verify_password_roundtrip() {
        let hash = hash_password("correct-horse-battery").expect("哈希密码失败");
        assert!(verify_password("correct-horse-battery", &hash));
        assert!(!verify_password("wrong-password", &hash));
    }

    #[test]
    fn hash_password_uses_random_salt() {
        let h1 = hash_password("same").expect("哈希密码失败");
        let h2 = hash_password("same").expect("哈希密码失败");
        assert_ne!(h1, h2);
        assert!(verify_password("same", &h1));
        assert!(verify_password("same", &h2));
    }

    #[test]
    fn verify_password_rejects_invalid_hash() {
        assert!(!verify_password("x", "not-a-valid-hash"));
    }
}

/// utils::jwt_util JWT 签发与校验
mod jwt_util {
    use super::*;

    #[test]
    fn access_token_roundtrip_preserves_claims() {
        install_test_jwt_keys();
        let token = generate_access_token("user-123".to_string(), "PC".to_string())
            .expect("签发 access token 失败");
        let claims = verify_token(&token).expect("校验 token 失败");
        assert_eq!(claims.uuid, "user-123");
        assert_eq!(claims.sub, "PC");
        assert!(claims.exp > get_now_time_stamp_as_secs().expect("获取当前秒级时间戳失败"));
    }

    #[test]
    fn token_with_expiry_verified() {
        install_test_jwt_keys();
        let token = generate_token_with_expiry("user-123".to_string(), "MOBILE".to_string(), 3600)
            .expect("签发带过期时间的 token 失败");
        let claims = verify_token(&token).expect("校验 token 失败");
        assert_eq!(claims.uuid, "user-123");
        assert_eq!(claims.sub, "MOBILE");
    }

    #[test]
    fn verify_token_rejects_tampered_token() {
        install_test_jwt_keys();
        let token = generate_access_token("user-123".to_string(), "PC".to_string())
            .expect("签发 access token 失败");

        let mut tampered = token.clone();
        tampered.push('x');
        assert!(verify_token(&tampered).is_err());

        let mid = token.len() / 2;
        let mut tampered_mid: Vec<char> = token.chars().collect();
        if tampered_mid[mid] != 'a' {
            tampered_mid[mid] = 'a';
        } else {
            tampered_mid[mid] = 'b';
        }
        assert!(verify_token(&tampered_mid.into_iter().collect::<String>()).is_err());
    }

    #[test]
    fn verify_token_rejects_malformed_token() {
        install_test_jwt_keys();
        assert!(verify_token("").is_err());
        assert!(verify_token("not-a-jwt").is_err());
        assert!(verify_token("a.b.c").is_err());
    }

    #[test]
    fn expired_token_is_rejected() {
        install_test_jwt_keys();
        let token = generate_token_with_expiry("user-123".to_string(), "PC".to_string(), -3600)
            .expect("签发已过期 token 失败");
        assert!(verify_token(&token).is_err());
    }

    #[test]
    fn access_token_expiry_is_about_24h() {
        install_test_jwt_keys();
        let now = get_now_time_stamp_as_secs().expect("获取当前秒级时间戳失败");
        let token = generate_access_token("user-123".to_string(), "PC".to_string())
            .expect("签发 access token 失败");
        let claims = verify_token(&token).expect("校验 token 失败");
        assert!(claims.exp > now + 3600 * 23, "exp 应约为 24h 后,实际 {}", claims.exp);
        assert!(claims.exp <= now + 3600 * 24 + 60, "exp 应不超过 24h 后,实际 {}", claims.exp);
    }

    #[test]
    fn token_with_custom_expiry_matches_requested_seconds() {
        install_test_jwt_keys();
        let now = get_now_time_stamp_as_secs().expect("获取当前秒级时间戳失败");
        let token = generate_token_with_expiry("user-123".to_string(), "MOBILE".to_string(), 600)
            .expect("签发带过期时间的 token 失败");
        let claims = verify_token(&token).expect("校验 token 失败");
        assert!(claims.exp > now + 500, "自定义过期时间应约为 600s,实际 {}", claims.exp);
        assert!(claims.exp <= now + 600 + 60, "自定义过期时间应不超过 600s,实际 {}", claims.exp);
    }
}

/// utils::server_count_sync 集群节点数 / 哈希取模
mod server_count_sync {
    use super::*;

    #[test]
    fn get_server_count_reflects_global() {
        SERVER_COUNT.store(1, Ordering::Relaxed);
        assert_eq!(get_server_count(), 1);

        SERVER_COUNT.store(7, Ordering::Relaxed);
        assert_eq!(get_server_count(), 7);

        SERVER_COUNT.store(1, Ordering::Relaxed);
        assert_eq!(get_server_count(), 1);
    }

    #[test]
    fn compute_preferred_index_is_deterministic_and_bounded() {
        // 单节点时恒为 0
        SERVER_COUNT.store(1, Ordering::Relaxed);
        assert_eq!(compute_preferred_index("uuid-1"), 0);
        assert_eq!(compute_preferred_index("uuid-2"), 0);

        // 多节点时结果确定且严格小于节点数
        SERVER_COUNT.store(5, Ordering::Relaxed);
        assert_eq!(compute_preferred_index("user-1"), compute_preferred_index("user-1"));
        for i in 0..200 {
            let idx = compute_preferred_index(&format!("user-{}", i));
            assert!(idx < 5, "index {} out of range for server_count 5", idx);
        }

        // 恢复默认
        SERVER_COUNT.store(1, Ordering::Relaxed);
    }
}

/// utils::internal_quic_client 内部 QUIC 客户端配置(跳过证书校验)
mod internal_quic_client {
    use super::*;

    #[tokio::test]
    async fn make_client_config_succeeds() {
        let config = make_internal_client_config().expect("构建内部 QUIC 客户端配置失败");
        // 配置可用:可挂载到 QUIC 客户端端点(仅绑定本地临时 UDP 端口,不发起网络连接)
        let mut endpoint = quinn::Endpoint::client("0.0.0.0:0".parse().expect("解析本地地址失败"))
            .expect("创建 QUIC 客户端端点失败");
        endpoint.set_default_client_config(config);
    }
}
