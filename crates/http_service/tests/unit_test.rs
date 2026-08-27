//! http_service 不依赖外部服务(DB/Redis/S3/网络)的单元测试
//!
//! 覆盖:HTTP 统一响应体、文件工具、DTO 校验逻辑、DTO/VO 转换与序列化。

use common::models::user_entity::basic_user::BasicUser;
use common::models::user_entity::user_info::UserInfo;
use http_service::common::dto::base_dto::ReqList;
use http_service::common::dto::base_page_dto::BasePageDTO;
use http_service::http_service::file_service::vo::biz_file_link_vo::BizFileLinkVO;
use http_service::http_service::file_service::vo::biz_record_vo::BizRecordVO;
use http_service::http_service::group_service::group_dto::add_member_dto::AddMemberDTO;
use http_service::http_service::group_service::group_dto::create_group_dto::CreateGroupDTO;
use http_service::http_service::group_service::group_dto::group_message_history_dto::GroupMessageHistoryDTO;
use http_service::http_service::group_service::group_dto::invite_member_dto::InviteMemberDTO;
use http_service::http_service::group_service::group_dto::set_role_dto::SetRoleDTO;
use http_service::http_service::group_service::group_dto::update_group_dto::UpdateGroupDTO;
use http_service::http_service::group_service::group_vo::group_info_vo::GroupInfoVO;
use http_service::http_service::group_service::group_vo::group_invitation_vo::GroupInvitationVO;
use http_service::http_service::group_service::group_vo::group_member_vo::GroupMemberVO;
use http_service::http_service::group_service::group_vo::group_message_vo::{
    GroupMessageVO, UnreadCountVO,
};
use http_service::http_service::moment_service::dto::moment_dto::{
    AddCommentDTO, CommentListQuery, CreateMomentDTO, DeleteMomentDTO, FollowToggleDTO,
    LikeToggleDTO, MomentListQuery,
};
use http_service::http_service::moment_service::vo::moment_vo::{
    MomentCommentListVO, MomentCommentVO, MomentListVO, MomentVO,
};
use http_service::http_service::user_service::dto::basic_user_dto::SignInBasicUserDTO;
use http_service::http_service::user_service::dto::complete_profile_dto::CompleteProfileDTO;
use http_service::http_service::user_service::dto::refresh_token_dto::RefreshTokenDTO;
use http_service::http_service::user_service::dto::send_verify_code_dto::SendVerifyCodeDTO;
use http_service::http_service::user_service::dto::sign_up_step1_dto::SignUpStep1DTO;
use http_service::http_service::user_service::dto::update_user_dto::UpdateUserDTO;
use http_service::utils::file_utils::{compress_image, get_image_mime_type, is_image_file};
use http_service::utils::http_response::{
    CommonResponse, CommonResponseNoDataRef, CommonResponseRef,
};
use rbatis::rbdc::Uuid;
use serde::Serialize;
use serde::de::DeserializeOwned;
use validator::Validate;

/// 通用 serde 往返断言(按序列化结果比较,不要求类型实现 PartialEq)
fn assert_roundtrip<T>(value: &T)
where
    T: Serialize + DeserializeOwned,
{
    let json = serde_json::to_value(value).expect("序列化失败");
    let back: T = serde_json::from_value(json.clone()).expect("反序列化失败");
    assert_eq!(json, serde_json::to_value(&back).expect("序列化失败"));
}

/// utils::http_response 统一响应体
mod http_response {
    use super::*;

    #[test]
    fn success_sets_200_and_success_message() {
        let resp = CommonResponse::success(42u32);
        assert_eq!(resp.code, 200);
        assert_eq!(resp.message, "Success");
        assert_eq!(resp.data, 42);
    }

    #[test]
    fn error_sets_500() {
        let resp = CommonResponse::error("x".to_string(), "boom".to_string());
        assert_eq!(resp.code, 500);
        assert_eq!(resp.message, "boom");
    }

    #[test]
    fn new_allows_custom_code() {
        let resp = CommonResponse::new(201, "created".to_string(), "done".to_string());
        assert_eq!(resp.code, 201);
        assert_eq!(resp.data, "created");
        assert_eq!(resp.message, "done");
    }

    #[test]
    fn success_json_produces_expected_fields() {
        let json = CommonResponse::success_json(7u32).expect("序列化失败");
        let v: serde_json::Value = serde_json::from_str(&json).expect("解析失败");
        assert_eq!(v["code"], 200);
        assert_eq!(v["data"], 7);
        assert_eq!(v["message"], "Success");
    }

    #[test]
    fn error_json_produces_expected_fields() {
        let json =
            CommonResponse::error_json("err".to_string(), "bad".to_string()).expect("序列化失败");
        let v: serde_json::Value = serde_json::from_str(&json).expect("解析失败");
        assert_eq!(v["code"], 500);
        assert_eq!(v["data"], "err");
        assert_eq!(v["message"], "bad");
    }

    #[test]
    fn response_ref_success() {
        let data = 42u32;
        let resp = CommonResponseRef::success(&data);
        assert_eq!(resp.code, 200);
        assert_eq!(resp.data, Some(&data));
        assert_eq!(resp.message, "Success");
    }

    #[test]
    fn response_ref_success_json() {
        let json = CommonResponseRef::<u32>::success_json(&42).expect("序列化失败");
        let v: serde_json::Value = serde_json::from_str(&json).expect("解析失败");
        assert_eq!(v["code"], 200);
        assert_eq!(v["data"], 42);
    }

    #[test]
    fn no_data_ref_error_json_uses_500() {
        let json = CommonResponseNoDataRef::error_json("oops");
        let v: serde_json::Value = serde_json::from_str(&json).expect("解析失败");
        assert_eq!(v["code"], 500);
        assert_eq!(v["data"], 0);
        assert_eq!(v["message"], "oops");
    }

    #[test]
    fn no_data_ref_success_empty_uses_204() {
        let json = CommonResponseNoDataRef::success_empty();
        let v: serde_json::Value = serde_json::from_str(&json).expect("解析失败");
        assert_eq!(v["code"], 204);
        assert_eq!(v["message"], "");
    }
}

/// utils::file_utils 图片类型判断
mod file_utils {
    use super::*;

    #[test]
    fn get_mime_type_for_known_extensions() {
        assert_eq!(get_image_mime_type("a.jpg"), Some("image/jpeg".to_string()));
        assert_eq!(get_image_mime_type("a.jpeg"), Some("image/jpeg".to_string()));
        assert_eq!(get_image_mime_type("a.png"), Some("image/png".to_string()));
        assert_eq!(get_image_mime_type("a.gif"), Some("image/gif".to_string()));
        assert_eq!(get_image_mime_type("a.webp"), Some("image/webp".to_string()));
        assert_eq!(get_image_mime_type("a.bmp"), Some("image/bmp".to_string()));
    }

    #[test]
    fn get_mime_type_ignores_case() {
        assert_eq!(get_image_mime_type("PHOTO.JPG"), Some("image/jpeg".to_string()));
        assert_eq!(get_image_mime_type("pic.PNG"), Some("image/png".to_string()));
    }

    #[test]
    fn get_mime_type_returns_none_for_unknown_or_missing_extension() {
        assert_eq!(get_image_mime_type("a.txt"), None);
        assert_eq!(get_image_mime_type("archive.zip"), None);
        assert_eq!(get_image_mime_type("noext"), None);
        assert_eq!(get_image_mime_type(""), None);
    }

    #[test]
    fn is_image_file_flags_images_only() {
        assert!(is_image_file("photo.png"));
        assert!(is_image_file("photo.gif"));
        assert!(!is_image_file("photo.pdf"));
        assert!(!is_image_file("noext"));
    }
}

/// utils::file_utils::compress_image 图片压缩
mod compress_image {
    use super::*;

    #[tokio::test]
    async fn returns_original_when_already_small() {
        let path =
            std::env::temp_dir().join(format!("unit_test_small_{}.txt", uuid::Uuid::new_v4()));
        std::fs::write(&path, b"hello world").expect("写入临时文件失败");
        let result = compress_image(path.to_str().expect("路径转字符串失败"), None)
            .await
            .expect("压缩小文件失败");
        assert_eq!(result, b"hello world");
        std::fs::remove_file(&path).ok();
    }

    #[tokio::test]
    async fn compresses_large_image_to_webp_smaller_than_original() {
        // 生成一张随机噪声大图(PNG 几乎无法压缩,必然超过默认 1MB 目标触发压缩路径)
        let mut img = image::RgbImage::new(800, 800);
        for px in img.pixels_mut() {
            *px = image::Rgb([rand::random(), rand::random(), rand::random()]);
        }
        let path = std::env::temp_dir().join(format!("unit_test_img_{}.png", uuid::Uuid::new_v4()));
        img.save(&path).expect("保存测试图片失败");
        let file_len = std::fs::metadata(&path).expect("读取文件元数据失败").len();

        let compressed = compress_image(path.to_str().expect("路径转字符串失败"), Some(128 * 1024))
            .await
            .expect("压缩大图失败");

        assert!(compressed.starts_with(b"RIFF"), "输出应为 WebP 格式");
        assert!(compressed.len() < file_len as usize, "压缩后应小于原文件");
        std::fs::remove_file(&path).ok();
    }
}

/// user_service DTO 校验
mod user_dto {
    use super::*;

    fn valid_step1() -> SignUpStep1DTO {
        SignUpStep1DTO {
            email: Some("a@b.com".to_string()),
            verification_code: Some("123456".to_string()),
        }
    }

    fn valid_complete_profile() -> CompleteProfileDTO {
        CompleteProfileDTO {
            reg_token: Some("reg-token-abc".to_string()),
            email: Some("a@b.com".to_string()),
            account: Some("acct001".to_string()),
            password: Some("abcdefghijklmn".to_string()),
            username: Some("user01".to_string()),
            icon: None,
            info: None,
        }
    }

    #[test]
    fn sign_up_step1_valid_passes() {
        assert!(valid_step1().validate().is_ok());
    }

    #[test]
    fn sign_up_step1_rejects_bad_email() {
        let mut dto = valid_step1();
        dto.email = Some("not-an-email".to_string());
        assert!(dto.validate().is_err());

        let mut dto = valid_step1();
        dto.email = None;
        assert!(dto.validate().is_err());
    }

    #[test]
    fn sign_up_step1_rejects_short_code() {
        let mut dto = valid_step1();
        dto.verification_code = Some("123".to_string());
        assert!(dto.validate().is_err());

        let mut dto = valid_step1();
        dto.verification_code = None;
        assert!(dto.validate().is_err());
    }

    #[test]
    fn complete_profile_valid_passes() {
        assert!(valid_complete_profile().validate().is_ok());
    }

    #[test]
    fn complete_profile_rejects_bad_or_missing_email() {
        let mut dto = valid_complete_profile();
        dto.email = Some("not-an-email".to_string());
        assert!(dto.validate().is_err());

        let mut dto = valid_complete_profile();
        dto.email = None;
        assert!(dto.validate().is_err());
    }

    #[test]
    fn complete_profile_rejects_short_account() {
        let mut dto = valid_complete_profile();
        dto.account = Some("ab".to_string());
        assert!(dto.validate().is_err());

        let mut dto = valid_complete_profile();
        dto.account = None;
        assert!(dto.validate().is_err());
    }

    #[test]
    fn complete_profile_rejects_weak_password() {
        let mut dto = valid_complete_profile();
        dto.password = Some("abc123".to_string());
        assert!(dto.validate().is_err());

        let mut dto = valid_complete_profile();
        dto.password = None;
        assert!(dto.validate().is_err());
    }

    #[test]
    fn complete_profile_rejects_missing_token_or_username() {
        let mut dto = valid_complete_profile();
        dto.reg_token = None;
        assert!(dto.validate().is_err());

        let mut dto = valid_complete_profile();
        dto.username = None;
        assert!(dto.validate().is_err());
    }

    #[test]
    fn sign_in_valid_passes() {
        let dto = SignInBasicUserDTO {
            account: Some("acct001".to_string()),
            password: Some("abcdefghijklmn".to_string()),
            platform: Some("PC".to_string()),
            device_fingerprint: Some("a1b2c3d4e5f60718293a4b5c6d7e8f90".to_string()),
        };
        assert!(dto.validate().is_ok());
    }

    #[test]
    fn sign_in_rejects_missing_fields() {
        let mut dto = SignInBasicUserDTO {
            account: Some("acct001".to_string()),
            password: Some("abcdefghijklmn".to_string()),
            platform: Some("PC".to_string()),
            device_fingerprint: Some("a1b2c3d4e5f60718293a4b5c6d7e8f90".to_string()),
        };
        dto.account = None;
        assert!(dto.validate().is_err());

        let mut dto = SignInBasicUserDTO {
            account: Some("acct001".to_string()),
            password: Some("abcdefghijklmn".to_string()),
            platform: Some("PC".to_string()),
            device_fingerprint: Some("a1b2c3d4e5f60718293a4b5c6d7e8f90".to_string()),
        };
        dto.platform = None;
        assert!(dto.validate().is_err());

        let mut dto = SignInBasicUserDTO {
            account: Some("acct001".to_string()),
            password: Some("abcdefghijklmn".to_string()),
            platform: Some("PC".to_string()),
            device_fingerprint: Some("a1b2c3d4e5f60718293a4b5c6d7e8f90".to_string()),
        };
        dto.device_fingerprint = None;
        assert!(dto.validate().is_err());

        let mut dto = SignInBasicUserDTO {
            account: Some("acct001".to_string()),
            password: Some("abcdefghijklmn".to_string()),
            platform: Some("PC".to_string()),
            device_fingerprint: Some("a1b2c3d4e5f60718293a4b5c6d7e8f90".to_string()),
        };
        dto.device_fingerprint = Some("short".to_string());
        assert!(dto.validate().is_err());
    }

    #[test]
    fn sign_in_to_basic_user_maps_fields() {
        let dto = SignInBasicUserDTO {
            account: Some("acct001".to_string()),
            password: Some("abcdefghijklmn".to_string()),
            platform: Some("PC".to_string()),
            device_fingerprint: Some("a1b2c3d4e5f60718293a4b5c6d7e8f90".to_string()),
        };
        let user = dto.to_basic_user();
        assert_eq!(user.account.as_deref(), Some("acct001"));
        assert_eq!(user.password.as_deref(), Some("abcdefghijklmn"));
        assert_eq!(user.username, None);
    }

    #[test]
    fn send_verify_code_valid_passes() {
        let dto = SendVerifyCodeDTO { email: Some("a@b.com".to_string()) };
        assert!(dto.validate().is_ok());
    }

    #[test]
    fn send_verify_code_rejects_bad_or_missing_email() {
        let dto = SendVerifyCodeDTO { email: Some("bad".to_string()) };
        assert!(dto.validate().is_err());

        let dto = SendVerifyCodeDTO { email: None };
        assert!(dto.validate().is_err());
    }

    #[test]
    fn refresh_token_non_empty_required() {
        let dto = RefreshTokenDTO {
            refresh_token: "token".to_string(),
            device_fingerprint: "a1b2c3d4e5f60718293a4b5c6d7e8f90".to_string(),
        };
        assert!(dto.validate().is_ok());

        let dto = RefreshTokenDTO {
            refresh_token: String::new(),
            device_fingerprint: "a1b2c3d4e5f60718293a4b5c6d7e8f90".to_string(),
        };
        assert!(dto.validate().is_err());

        let dto = RefreshTokenDTO {
            refresh_token: "token".to_string(),
            device_fingerprint: "short".to_string(),
        };
        assert!(dto.validate().is_err());
    }

    fn valid_update() -> UpdateUserDTO {
        UpdateUserDTO {
            username: Some("nick".to_string()),
            info: Some("hello".to_string()),
            gender: Some(2),
            age: Some(30),
            birthday: Some(1234567890),
            phone: Some("13800138000".to_string()),
            email: Some("a@b.com".to_string()),
            address: Some("beijing".to_string()),
        }
    }

    #[test]
    fn update_user_valid_passes() {
        assert!(valid_update().validate().is_ok());
    }

    #[test]
    fn update_user_rejects_bad_phone_and_email() {
        let mut dto = valid_update();
        dto.phone = Some("123".to_string());
        assert!(dto.validate().is_err());

        let mut dto = valid_update();
        dto.email = Some("bad".to_string());
        assert!(dto.validate().is_err());
    }

    #[test]
    fn update_user_rejects_oversized_fields() {
        let mut dto = valid_update();
        dto.username = Some("x".repeat(51));
        assert!(dto.validate().is_err());

        let mut dto = valid_update();
        dto.info = Some("x".repeat(201));
        assert!(dto.validate().is_err());

        let mut dto = valid_update();
        dto.address = Some("x".repeat(201));
        assert!(dto.validate().is_err());
    }

    #[test]
    fn update_user_apply_to_basic_user_updates_only_present_fields() {
        let dto = UpdateUserDTO {
            username: Some("newname".to_string()),
            info: None,
            gender: Some(2),
            age: Some(30),
            birthday: None,
            phone: None,
            email: None,
            address: None,
        };
        let mut user = BasicUser {
            uuid: None,
            username: Some("old".to_string()),
            account: Some("acct".to_string()),
            icon: Some("icon".to_string()),
            info: Some("oldinfo".to_string()),
            password: None,
            registration_status: Some(1),
        };
        dto.apply_to_basic_user(&mut user);
        assert_eq!(user.username.as_deref(), Some("newname"));
        assert_eq!(user.info.as_deref(), Some("oldinfo"), "info 未提供时应保持不变");
        assert_eq!(user.account.as_deref(), Some("acct"));
    }

    #[test]
    fn update_user_apply_to_user_info_updates_fields_and_timestamp() {
        let dto = valid_update();
        let mut info = UserInfo {
            uuid: None,
            gender: None,
            age: None,
            birthday: None,
            note: None,
            created_at: Some(1),
            updated_at: Some(1),
            phone: None,
            email: None,
            address: None,
            status: Some(0),
        };
        dto.apply_to_user_info(&mut info).expect("更新 user_info 失败");
        assert_eq!(info.gender, Some(2));
        assert_eq!(info.age, Some(30));
        assert_eq!(info.birthday, Some(1234567890));
        assert_eq!(info.phone.as_deref(), Some("13800138000"));
        assert_eq!(info.email.as_deref(), Some("a@b.com"));
        assert_eq!(info.address.as_deref(), Some("beijing"));
        assert!(info.updated_at.unwrap_or(0) >= 1, "updated_at 应被刷新");
        assert_eq!(info.status, Some(0), "未提供的字段应保持不变");
    }
}

/// group_service DTO 校验
mod group_dto {
    use super::*;

    #[test]
    fn create_group_valid_passes() {
        let dto = CreateGroupDTO {
            group_name: "技术群".to_string(),
            avatar: None,
            description: None,
            max_members: Some(100),
        };
        assert!(dto.validate().is_ok());
    }

    #[test]
    fn create_group_rejects_empty_or_oversized_name() {
        let dto = CreateGroupDTO {
            group_name: String::new(),
            avatar: None,
            description: None,
            max_members: None,
        };
        assert!(dto.validate().is_err());

        let dto = CreateGroupDTO {
            group_name: "x".repeat(101),
            avatar: None,
            description: None,
            max_members: None,
        };
        assert!(dto.validate().is_err());
    }

    #[test]
    fn create_group_rejects_oversized_avatar_and_description() {
        let dto = CreateGroupDTO {
            group_name: "群".to_string(),
            avatar: Some("a".repeat(501)),
            description: None,
            max_members: None,
        };
        assert!(dto.validate().is_err());

        let dto = CreateGroupDTO {
            group_name: "群".to_string(),
            avatar: None,
            description: Some("a".repeat(501)),
            max_members: None,
        };
        assert!(dto.validate().is_err());
    }

    #[test]
    fn add_member_requires_non_empty_list() {
        let dto = AddMemberDTO { group_uuid: "g1".to_string(), user_uuids: vec!["u1".to_string()] };
        assert!(dto.validate().is_ok());

        let dto = AddMemberDTO { group_uuid: "g1".to_string(), user_uuids: vec![] };
        assert!(dto.validate().is_err());
    }

    #[test]
    fn invite_member_requires_non_empty_list() {
        let dto =
            InviteMemberDTO { group_uuid: "g1".to_string(), user_uuids: vec!["u1".to_string()] };
        assert!(dto.validate().is_ok());

        let dto = InviteMemberDTO { group_uuid: "g1".to_string(), user_uuids: vec![] };
        assert!(dto.validate().is_err());
    }

    #[test]
    fn set_role_validates_range() {
        for role in [0i16, 1, 2] {
            let dto =
                SetRoleDTO { group_uuid: "g1".to_string(), user_uuid: "u1".to_string(), role };
            assert!(dto.validate().is_ok(), "role={} 应在 0-2 内", role);
        }

        for role in [-1i16, 3] {
            let dto =
                SetRoleDTO { group_uuid: "g1".to_string(), user_uuid: "u1".to_string(), role };
            assert!(dto.validate().is_err(), "role={} 应超出范围", role);
        }
    }

    #[test]
    fn group_message_history_valid_passes() {
        let dto = GroupMessageHistoryDTO {
            group_uuid: "g1".to_string(),
            start: Some(0),
            size: Some(50),
            last_read_msg_id: Some(100),
        };
        assert!(dto.validate().is_ok());
    }

    #[test]
    fn group_message_history_rejects_out_of_range() {
        let dto = GroupMessageHistoryDTO {
            group_uuid: "g1".to_string(),
            start: None,
            size: Some(0),
            last_read_msg_id: None,
        };
        assert!(dto.validate().is_err());

        let dto = GroupMessageHistoryDTO {
            group_uuid: "g1".to_string(),
            start: None,
            size: Some(101),
            last_read_msg_id: None,
        };
        assert!(dto.validate().is_err());

        let dto = GroupMessageHistoryDTO {
            group_uuid: "g1".to_string(),
            start: None,
            size: None,
            last_read_msg_id: Some(-1),
        };
        assert!(dto.validate().is_err());
    }

    #[test]
    fn update_group_validation() {
        let dto = UpdateGroupDTO {
            group_uuid: "g1".to_string(),
            group_name: Some("新名字".to_string()),
            avatar: None,
            description: None,
        };
        assert!(dto.validate().is_ok());

        let dto = UpdateGroupDTO {
            group_uuid: "g1".to_string(),
            group_name: Some(String::new()),
            avatar: None,
            description: None,
        };
        assert!(dto.validate().is_err());
    }
}

/// DTO/VO 序列化往返
mod serde_roundtrip {
    use super::*;

    #[test]
    fn base_dto_roundtrip() {
        assert_roundtrip(&BasePageDTO { page_num: Some(1), page_size: Some(20), total: Some(99) });
        assert_roundtrip(&BasePageDTO { page_num: None, page_size: None, total: None });
    }

    #[test]
    fn req_list_roundtrip() {
        assert_roundtrip(&ReqList::<String> {
            page_num: Some(1),
            page_size: Some(10),
            data: Some("data".to_string()),
        });
        assert_roundtrip(&ReqList::<String> { page_num: None, page_size: None, data: None });
    }

    #[test]
    fn group_vo_roundtrip() {
        assert_roundtrip(&GroupInfoVO {
            group_uuid: "g1".to_string(),
            group_name: "群".to_string(),
            avatar: Some("a".to_string()),
            owner_uuid: "u1".to_string(),
            description: Some("desc".to_string()),
            max_members: 100,
            member_count: 5,
            created_at: 1,
            updated_at: 2,
            status: 0,
        });
        assert_roundtrip(&GroupMemberVO {
            user_uuid: "u1".to_string(),
            role: 0,
            nickname: Some("nick".to_string()),
            join_time: 1,
            muted: false,
            status: 0,
        });
        assert_roundtrip(&GroupInvitationVO {
            id: 1,
            group_uuid: "g1".to_string(),
            group_name: "群".to_string(),
            group_avatar: None,
            inviter_uuid: "u1".to_string(),
            invitee_uuid: "u2".to_string(),
            status: 0,
            created_at: 1,
        });
        assert_roundtrip(&GroupMessageVO {
            nano_id: "nano-1".to_string(),
            group_uuid: "g1".to_string(),
            send_user: "u1".to_string(),
            timestamp: 1,
            raw: b"hello".to_vec(),
            msg_type: 2001,
            recalled: false,
        });
        assert_roundtrip(&UnreadCountVO {
            group_uuid: "g1".to_string(),
            unread_count: 3,
            last_read_msg_id: 9,
        });
    }

    #[test]
    fn file_vo_roundtrip() {
        let uuid = Uuid("biz-1".to_string());
        assert_roundtrip(&BizFileLinkVO {
            biz_id: Some(uuid.clone()),
            origin_file_id: None,
            file_id: Some(uuid),
        });
        assert_roundtrip(&BizRecordVO {
            uuid: Some(Uuid("rec-1".to_string())),
            biz_name: Some("avatar".to_string()),
            description: None,
            biz_type: Some("USER_AVATAR".to_string()),
            remark: None,
            file_infos: None,
        });
    }
}

/// 动态广场 DTO/VO 序列化与字段映射
mod moment {
    use super::*;

    #[test]
    fn moment_dto_roundtrip() {
        assert_roundtrip(&CreateMomentDTO {
            content: "今日份美好".to_string(),
            visibility: 0,
            file_ids: vec!["f1".to_string(), "f2".to_string()],
        });
        assert_roundtrip(&LikeToggleDTO { moment_uuid: "m1".to_string() });
        assert_roundtrip(&DeleteMomentDTO { moment_uuid: "m1".to_string() });
        assert_roundtrip(&AddCommentDTO {
            moment_uuid: "m1".to_string(),
            content: "写得好".to_string(),
        });
        assert_roundtrip(&CommentListQuery { moment_uuid: "m1".to_string() });
    }

    #[test]
    fn moment_vo_roundtrip() {
        assert_roundtrip(&MomentVO {
            uuid: "m1".to_string(),
            author_uuid: "u1".to_string(),
            username: Some("Alice".to_string()),
            icon: Some("icon-1".to_string()),
            content: "内容".to_string(),
            visibility: 0,
            image_count: 3,
            like_count: 3,
            comment_count: 2,
            liked_by_me: true,
            followed_by_me: false,
            created_at: 1_700_000_000,
            updated_at: 1_700_000_001,
        });
    }

    #[test]
    fn moment_list_vo_serializes_fields() {
        let vo = MomentListVO {
            total: 1,
            list: vec![MomentVO {
                uuid: "m1".to_string(),
                author_uuid: "u1".to_string(),
                username: None,
                icon: None,
                content: "内容".to_string(),
                visibility: 1,
                image_count: 2,
                like_count: 0,
                comment_count: 0,
                liked_by_me: false,
                followed_by_me: false,
                created_at: 1,
                updated_at: 1,
            }],
        };
        let value = serde_json::to_value(&vo).expect("序列化失败");
        assert_eq!(value["total"], 1);
        assert_eq!(value["list"][0]["uuid"], "m1");
        assert_eq!(value["list"][0]["visibility"], 1);
        assert_eq!(value["list"][0]["image_count"], 2);
        assert_eq!(value["list"][0]["liked_by_me"], false);
    }

    #[test]
    fn moment_comment_vo_roundtrip() {
        assert_roundtrip(&MomentCommentVO {
            id: "c1".to_string(),
            moment_uuid: "m1".to_string(),
            author_uuid: "u2".to_string(),
            username: Some("Bob".to_string()),
            icon: None,
            content: "哈哈".to_string(),
            created_at: 1_700_000_000,
        });
    }

    #[test]
    fn moment_comment_list_vo_serializes_fields() {
        let vo = MomentCommentListVO {
            total: 1,
            list: vec![MomentCommentVO {
                id: "c1".to_string(),
                moment_uuid: "m1".to_string(),
                author_uuid: "u2".to_string(),
                username: None,
                icon: Some("icon-2".to_string()),
                content: "哈哈".to_string(),
                created_at: 1,
            }],
        };
        let value = serde_json::to_value(&vo).expect("序列化失败");
        assert_eq!(value["total"], 1);
        assert_eq!(value["list"][0]["id"], "c1");
        assert_eq!(value["list"][0]["icon"], "icon-2");
    }
}

/// 工具宏
mod macros {
    #[test]
    fn serde_json_to_string_serializes_success() {
        // 手动构造 Value,避免 serde_json::json! 宏内部使用被禁用的 unwrap
        let value = serde_json::Value::Object(serde_json::Map::from_iter([(
            "k".to_string(),
            serde_json::Value::from(1),
        )]));
        let result: Result<String, String> = http_service::serde_json_to_string!(&value);
        let json = result.expect("序列化失败");
        let v: serde_json::Value = serde_json::from_str(&json).expect("解析失败");
        assert_eq!(v["k"], 1);
    }
}
