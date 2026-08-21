//! entity 单元测试
//!
//! 模型均不依赖外部服务(数据库/Redis),仅测试结构体构造、
//! serde 序列化往返与常量定义。

use std::str::FromStr;

use rbatis::rbdc::{Bytes, Uuid};
use validator::Validate;

use crate::models::chat_entity::add_read_chat_record::AddReadChatRecordDTO;
use crate::models::chat_entity::chat_list_link::ChatListLink;
use crate::models::chat_entity::chat_message_read::{
    CHAT_TYPE_GROUP, CHAT_TYPE_SINGLE, ChatMessageRecordRead,
};
use crate::models::chat_entity::chat_message_record::ChatMessageRecord;
use crate::models::file_entity::biz_file_link::BizFileLink;
use crate::models::file_entity::biz_record::BizRecord;
use crate::models::file_entity::chat_biz_record::ChatBizRecord;
use crate::models::file_entity::file_upload_record::FileUploadRecord;
use crate::models::file_entity::private_biz_record::PrivateBizRecord;
use crate::models::group_entity::group_info::GroupInfo;
use crate::models::group_entity::group_invitation::{
    GroupInvitation, INVITATION_ACCEPTED, INVITATION_DECLINED, INVITATION_PENDING,
};
use crate::models::group_entity::group_member::{
    GroupMember, ROLE_ADMIN, ROLE_MEMBER, ROLE_OWNER, STATUS_KICKED, STATUS_NORMAL, STATUS_QUIT,
};
use crate::models::group_entity::group_message_read::GroupMessageRecordRead;
use crate::models::group_entity::group_message_record::{
    GroupMessageRecord, MSG_TYPE_FILE, MSG_TYPE_IMAGE, MSG_TYPE_TEXT,
};
use crate::models::notify_entity::system_notification::SystemNotification;
use crate::models::user_entity::basic_user::BasicUser;
use crate::models::user_entity::email_sso::EmailSso;
use crate::models::user_entity::friend_link::FriendLink;
use crate::models::user_entity::friend_request_info::FriendRequestInfo;
use crate::models::user_entity::user_info::UserInfo;
use crate::models::user_entity::user_login_log::UserLoginLog;

/// 构造 rbdc Uuid(固定值保证断言确定)
fn uuid(s: &str) -> Uuid {
    Uuid::from_str(s).expect("解析 Uuid 失败")
}

/// 序列化 -> 反序列化 往返后,JSON 内容应保持一致
fn assert_roundtrip<T>(v: &T)
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let json = serde_json::to_value(v).expect("序列化失败");
    let de: T = serde_json::from_value(json.clone()).expect("反序列化失败");
    assert_eq!(serde_json::to_value(&de).expect("序列化失败"), json);
}

/// 常量定义
mod constants {
    use super::*;

    #[test]
    fn chat_type_constants() {
        assert_eq!(CHAT_TYPE_SINGLE, 1);
        assert_eq!(CHAT_TYPE_GROUP, 2);
    }

    #[test]
    fn group_invitation_constants() {
        assert_eq!(INVITATION_PENDING, 1);
        assert_eq!(INVITATION_ACCEPTED, 2);
        assert_eq!(INVITATION_DECLINED, 3);
    }

    #[test]
    fn group_member_role_constants() {
        assert_eq!(ROLE_MEMBER, 0);
        assert_eq!(ROLE_ADMIN, 1);
        assert_eq!(ROLE_OWNER, 2);
    }

    #[test]
    fn group_member_status_constants() {
        assert_eq!(STATUS_NORMAL, 1);
        assert_eq!(STATUS_QUIT, 2);
        assert_eq!(STATUS_KICKED, 3);
    }

    #[test]
    fn group_message_type_constants() {
        assert_eq!(MSG_TYPE_TEXT, 1);
        assert_eq!(MSG_TYPE_IMAGE, 2);
        assert_eq!(MSG_TYPE_FILE, 3);
    }
}

/// 用户模块
mod user_entity {
    use super::*;

    #[test]
    fn basic_user_roundtrip() {
        let user = BasicUser {
            uuid: Some(uuid("00000000-0000-0000-0000-000000000001")),
            username: Some("alice".to_string()),
            account: Some("alice001".to_string()),
            icon: Some("icon.png".to_string()),
            info: Some("hello".to_string()),
            password: Some("secret".to_string()),
            registration_status: Some(1),
        };
        assert_roundtrip(&user);
    }

    #[test]
    fn basic_user_missing_optional_fields_defaults_to_none() {
        let user: BasicUser = serde_json::from_str("{}").expect("反序列化失败");
        assert!(user.uuid.is_none());
        assert!(user.username.is_none());
        assert!(user.account.is_none());
        assert!(user.password.is_none());
        assert!(user.registration_status.is_none());
    }

    #[test]
    fn basic_user_validate_ok() {
        let user = BasicUser {
            uuid: Some(uuid("00000000-0000-0000-0000-000000000001")),
            username: Some("alice".to_string()),
            account: Some("alice001".to_string()),
            icon: None,
            info: None,
            password: Some("secret".to_string()),
            registration_status: Some(1),
        };
        assert!(user.validate().is_ok());
    }

    #[test]
    fn email_sso_roundtrip() {
        let sso = EmailSso {
            uuid: Some(uuid("00000000-0000-0000-0000-000000000001")),
            email: Some("alice@example.com".to_string()),
            email_normalized: Some("alice@example.com".to_string()),
            verified: Some(true),
            verified_at: Some(1_700_000_000),
            verify_code_issued_at: Some(1_700_000_000),
            is_primary: Some(true),
            status: Some(1),
            last_login_at: None,
            last_login_ip: None,
            login_count: Some(0),
            fail_count: Some(0),
            locked_until: None,
            created_at: Some(1_700_000_000),
            updated_at: Some(1_700_000_001),
            deleted_at: None,
        };
        assert_roundtrip(&sso);
    }

    #[test]
    fn user_info_roundtrip() {
        let info = UserInfo {
            uuid: Some(uuid("00000000-0000-0000-0000-000000000002")),
            gender: Some(1),
            age: Some(28),
            birthday: Some(946684800),
            note: Some("note".to_string()),
            created_at: Some(1_700_000_000),
            updated_at: Some(1_700_000_001),
            phone: Some("13800000000".to_string()),
            email: Some("u@example.com".to_string()),
            address: Some("addr".to_string()),
            status: Some(0),
        };
        assert_roundtrip(&info);
    }

    #[test]
    fn user_login_log_roundtrip() {
        let log = UserLoginLog {
            id: Some(1),
            uuid: Some(uuid("00000000-0000-0000-0000-000000000001")),
            account: Some("alice001".to_string()),
            login_type: Some("account".to_string()),
            event_type: Some("success".to_string()),
            login_at: Some(1_700_000_000),
            platform: Some("PC".to_string()),
            ipv4: Some("127.0.0.1".to_string()),
            ipv6: None,
            user_agent: Some("Mozilla/5.0".to_string()),
            device: None,
            result: None,
        };
        assert_roundtrip(&log);
    }

    #[test]
    fn friend_link_roundtrip() {
        let link = FriendLink {
            uuid: Some(uuid("00000000-0000-0000-0000-000000000003")),
            request_user: Some(uuid("00000000-0000-0000-0000-000000000001")),
            accept_user: Some(uuid("00000000-0000-0000-0000-000000000002")),
            is_del: Some(false),
            created_at: Some(1_700_000_000),
            updated_at: Some(1_700_000_001),
            version: Some(0),
        };
        assert_roundtrip(&link);
    }

    #[test]
    fn friend_request_info_roundtrip() {
        let info = FriendRequestInfo {
            id: Some(1),
            uuid: Some(uuid("00000000-0000-0000-0000-000000000004")),
            accept_status: Some(0),
            created_at: Some(1_700_000_000),
            updated_at: Some(1_700_000_001),
            request_message: Some("hi".to_string()),
            accept_message: None,
            request_user: Some(uuid("00000000-0000-0000-0000-000000000001")),
            accept_user: Some(uuid("00000000-0000-0000-0000-000000000002")),
            add_type: Some("search".to_string()),
            version: Some(0),
        };
        assert_roundtrip(&info);
    }
}

/// 会话模块
mod chat_entity {
    use super::*;

    #[test]
    fn chat_list_link_roundtrip() {
        let link = ChatListLink {
            id: Some(1),
            uuid: uuid("00000000-0000-0000-0000-000000000010"),
            friend_uuid: uuid("00000000-0000-0000-0000-000000000011"),
            created_at: Some(1_700_000_000),
            enable: Some(true),
        };
        assert_roundtrip(&link);
    }

    #[test]
    fn chat_message_read_roundtrip() {
        let record = ChatMessageRecordRead {
            id: Some(1),
            nano_id: Some("nano-1".to_string()),
            timestamp: Some(1_700_000_000),
            send_user: uuid("00000000-0000-0000-0000-000000000001"),
            recv_user: uuid("00000000-0000-0000-0000-000000000002"),
        };
        assert_roundtrip(&record);
    }

    #[test]
    fn chat_message_record_roundtrip_and_raw_bytes_serialize_as_array() {
        let record = ChatMessageRecord {
            id: Some(1),
            nano_id: Some("nano-1".to_string()),
            timestamp: Some(1_700_000_000),
            raw: Bytes::from(vec![0x48, 0x69]),
            text_type: Some(1),
            send_user: uuid("00000000-0000-0000-0000-000000000001"),
            recv_user: uuid("00000000-0000-0000-0000-000000000002"),
        };
        assert_roundtrip(&record);

        let json = serde_json::to_value(&record).expect("序列化失败");
        assert_eq!(
            json["raw"],
            serde_json::from_str::<serde_json::Value>("[72, 105]").expect("解析预期 JSON 失败")
        );
    }

    #[test]
    fn add_read_chat_record_dto_chat_type_defaults_to_none() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{"nano_id":"nano-1","timestamp":1700000000000,"send_user":"00000000-0000-0000-0000-000000000001","recv_user":"00000000-0000-0000-0000-000000000002"}"#,
        )
        .expect("解析 JSON 失败");
        let dto: AddReadChatRecordDTO = serde_json::from_value(json).expect("反序列化失败");
        assert_eq!(dto.nano_id.as_deref(), Some("nano-1"));
        assert_eq!(dto.chat_type, None);
    }

    #[test]
    fn add_read_chat_record_dto_with_chat_type() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{"send_user":"00000000-0000-0000-0000-000000000001","recv_user":"00000000-0000-0000-0000-000000000002","chat_type":2}"#,
        )
        .expect("解析 JSON 失败");
        let dto: AddReadChatRecordDTO = serde_json::from_value(json).expect("反序列化失败");
        assert_eq!(dto.chat_type, Some(2));
    }
}

/// 文件模块
mod file_entity {
    use super::*;

    #[test]
    fn biz_file_link_roundtrip() {
        let link = BizFileLink {
            id: Some(1),
            biz_id: Some(uuid("00000000-0000-0000-0000-000000000020")),
            origin_file_id: Some(uuid("00000000-0000-0000-0000-000000000021")),
            file_id: Some(uuid("00000000-0000-0000-0000-000000000022")),
            is_del: Some(false),
        };
        assert_roundtrip(&link);
    }

    #[test]
    fn biz_record_roundtrip() {
        let record = BizRecord {
            id: Some(1),
            uuid: Some(uuid("00000000-0000-0000-0000-000000000023")),
            biz_name: Some("avatar".to_string()),
            description: Some("desc".to_string()),
            created_by: Some(uuid("00000000-0000-0000-0000-000000000001")),
            created_at: Some(1_700_000_000),
            updated_at: Some(1_700_000_001),
            status: Some(0),
            approve_status: Some(1),
            biz_type: Some("avatar".to_string()),
            remark: Some("remark".to_string()),
        };
        assert_roundtrip(&record);
    }

    #[test]
    fn chat_biz_record_roundtrip() {
        let record = ChatBizRecord {
            id: Some(1),
            uuid: Some(uuid("00000000-0000-0000-0000-000000000024")),
            biz_name: Some("chat".to_string()),
            description: Some("desc".to_string()),
            created_by: Some(uuid("00000000-0000-0000-0000-000000000001")),
            receiver: Some(uuid("00000000-0000-0000-0000-000000000002")),
            created_at: Some(1_700_000_000),
            updated_at: Some(1_700_000_001),
            status: Some(0),
            approve_status: Some(1),
            biz_type: Some("single".to_string()),
            remark: Some("remark".to_string()),
        };
        assert_roundtrip(&record);
    }

    #[test]
    fn file_upload_record_roundtrip() {
        let record = FileUploadRecord {
            id: Some(1),
            uuid: Some(uuid("00000000-0000-0000-0000-000000000025")),
            original_name: Some("a.png".to_string()),
            stored_name: Some("stored.png".to_string()),
            file_path: Some("bucket/key".to_string()),
            file_size: Some(1024),
            mime_type: Some("image/png".to_string()),
            file_hash: Some("abc123".to_string()),
            upload_user_uuid: Some(uuid("00000000-0000-0000-0000-000000000001")),
            upload_time: Some(1_700_000_000),
            status: Some(0),
            description: Some("desc".to_string()),
            download_count: Some(3),
            last_download_time: Some(1_700_000_100),
            is_oss: Some(1),
            oss_type: Some(0),
            bucket: Some("only-talk-rs".to_string()),
        };
        assert_roundtrip(&record);
    }

    #[test]
    fn file_upload_record_empty_has_all_fields_none() {
        let record = FileUploadRecord::empty();
        assert!(record.id.is_none());
        assert!(record.uuid.is_none());
        assert!(record.original_name.is_none());
        assert!(record.stored_name.is_none());
        assert!(record.file_path.is_none());
        assert!(record.file_size.is_none());
        assert!(record.mime_type.is_none());
        assert!(record.file_hash.is_none());
        assert!(record.upload_user_uuid.is_none());
        assert!(record.upload_time.is_none());
        assert!(record.status.is_none());
        assert!(record.description.is_none());
        assert!(record.download_count.is_none());
        assert!(record.last_download_time.is_none());
        assert!(record.is_oss.is_none());
        assert!(record.oss_type.is_none());
        assert!(record.bucket.is_none());
    }

    #[test]
    fn private_biz_record_roundtrip() {
        let record = PrivateBizRecord {
            id: Some(1),
            uuid: Some(uuid("00000000-0000-0000-0000-000000000026")),
            policy_id: Some("policy-1".to_string()),
            biz_name: Some("moment".to_string()),
            description: Some("desc".to_string()),
            created_by: Some(uuid("00000000-0000-0000-0000-000000000001")),
            created_at: Some(1_700_000_000),
            updated_at: Some(1_700_000_001),
            status: Some(0),
            approve_status: Some(1),
            biz_type: Some("moment".to_string()),
            remark: Some("remark".to_string()),
        };
        assert_roundtrip(&record);
    }
}

/// 群组模块
mod group_entity {
    use super::*;

    #[test]
    fn group_info_roundtrip() {
        let info = GroupInfo {
            id: Some(1),
            group_uuid: Some(uuid("00000000-0000-0000-0000-000000000030")),
            group_name: Some("rust group".to_string()),
            avatar: Some("g.png".to_string()),
            owner_uuid: Some(uuid("00000000-0000-0000-0000-000000000001")),
            description: Some("desc".to_string()),
            max_members: Some(200),
            created_at: Some(1_700_000_000),
            updated_at: Some(1_700_000_001),
            status: Some(1),
        };
        assert_roundtrip(&info);
    }

    #[test]
    fn group_invitation_roundtrip() {
        let invitation = GroupInvitation {
            id: Some(1),
            group_uuid: Some(uuid("00000000-0000-0000-0000-000000000030")),
            inviter_uuid: Some(uuid("00000000-0000-0000-0000-000000000001")),
            invitee_uuid: Some(uuid("00000000-0000-0000-0000-000000000002")),
            status: Some(INVITATION_PENDING),
            created_at: Some(1_700_000_000),
            updated_at: Some(1_700_000_001),
        };
        assert_roundtrip(&invitation);
    }

    #[test]
    fn group_member_roundtrip() {
        let member = GroupMember {
            id: Some(1),
            group_uuid: Some(uuid("00000000-0000-0000-0000-000000000030")),
            user_uuid: Some(uuid("00000000-0000-0000-0000-000000000001")),
            role: Some(ROLE_OWNER),
            nickname: Some("alice".to_string()),
            join_time: Some(1_700_000_000),
            last_read_msg_id: Some(0),
            muted: Some(false),
            status: Some(STATUS_NORMAL),
        };
        assert_roundtrip(&member);
    }

    #[test]
    fn group_message_read_roundtrip() {
        let record = GroupMessageRecordRead {
            id: Some(1),
            nano_id: Some("nano-1".to_string()),
            timestamp: Some(1_700_000_000),
            send_user: uuid("00000000-0000-0000-0000-000000000001"),
            group_uuid: uuid("00000000-0000-0000-0000-000000000030"),
            read_user: uuid("00000000-0000-0000-0000-000000000002"),
        };
        assert_roundtrip(&record);
    }

    #[test]
    fn group_message_record_roundtrip() {
        let record = GroupMessageRecord {
            id: Some(1),
            nano_id: Some("nano-1".to_string()),
            group_uuid: Some(uuid("00000000-0000-0000-0000-000000000030")),
            send_user: Some(uuid("00000000-0000-0000-0000-000000000001")),
            timestamp: Some(1_700_000_000),
            raw: Bytes::from(b"hello".to_vec()),
            msg_type: Some(MSG_TYPE_TEXT),
            recalled: Some(false),
        };
        assert_roundtrip(&record);
    }
}

/// 通知模块
mod notify_entity {
    use super::*;

    #[test]
    fn system_notification_roundtrip() {
        let notification = SystemNotification {
            id: Some(uuid("00000000-0000-0000-0000-000000000040")),
            title: Some("title".to_string()),
            content: Some("content".to_string()),
            created_at: Some(1_700_000_000),
            content_type: Some(0),
            user_id: Some(uuid("00000000-0000-0000-0000-000000000001")),
            is_read: Some(false),
            biz_id: Some("biz-1".to_string()),
            level1: Some(1),
            level2: Some(2),
            level3: Some(3),
            level4: Some(4),
            unread_count: Some(5),
            priority: Some(1),
        };
        assert_roundtrip(&notification);
    }
}
