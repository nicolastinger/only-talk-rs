//! http_service 集成测试：在独立的测试数据库与测试 Redis 上启动真实 Actix HTTP 服务，
//! 覆盖用户模块的注册/登录/查询/刷新 token 等接口。
//!
//! - 测试数据库：复用仓库根目录 `.env` 的 `DATABASE_URL`（管理员连接）创建 `only_talk_http_test`，
//!   执行 `entity/ddl` 全部建表脚本，测试结束后 **删除** 该测试库。
//! - 测试 Redis：读取 `.env` 的 `TEST_REDIS_URL`（建议独立 DB index/实例），
//!   测试结束后 **清空该测试 Redis 的所有 key**（FLUSHALL）。
//!
//! 运行方式：
//!   cargo test -p http_service --test http_service_integration_test -- --ignored
//! 前提：本地 PostgreSQL、Redis 可用，且仓库根目录存在 `.env`。

use std::sync::Arc;
use std::time::Duration;

use actix_web::http::StatusCode;
use actix_web::middleware::from_fn;
use actix_web::{App, test, web};
use anyhow::{Context, Result, anyhow};
use common::config_str::{EMAIL_VERIFY_CODE, REGISTER_SESSION_TOKEN};
use common::models::chat_entity::chat_message_record::ChatMessageRecord;
use common::models::group_entity::group_info::GroupInfo;
use common::models::group_entity::group_message_record::{GroupMessageRecord, MSG_TYPE_TEXT};
use common::models::moment_entity::moment::Moment;
use common::models::moment_entity::moment_comment::MomentComment;
use common::models::plaza_entity::plaza_user_info::PlazaUserInfo;
use common::models::report_entity::report::Report;
use common::models::session_entity::aggregate::aggregate_user_sessions;
use common::models::session_entity::session::{SESSION_TYPE_GROUP, SESSION_TYPE_SINGLE, Session};
use common::models::session_entity::user_session::UserSession;
use common::models::user_entity::basic_user::BasicUser;
use common::models::user_entity::email_sso::EmailSso;
use common::models::user_entity::friend_link::FriendLink;
use common::models::user_entity::friend_request_info::FriendRequestInfo;
use common::models::user_entity::user_info::UserInfo;
use common::state::CoreState;
use common::utils::rsa_util::hash_password;
use common::utils::session_uuid::single_session_uuid;
use common::utils::time::get_now_time_stamp_as_millis;
use deadpool_redis::redis::{AsyncCommands, cmd};
use deadpool_redis::{Config as RedisConfig, Pool, Runtime};
use email_service::config::EmailServiceConfig;
use email_service::manager::EmailManager;
use futures_util::FutureExt;
use http_service::http_service::configure_routes;
use http_service::http_service::session_service::dto::{
    SessionControlDTO, SessionListCursor, SessionListDTO, SyncRequestDTO, SyncSessionReq,
};
use http_service::http_service::session_service::service::{
    delete_session, list_sessions, mute_session, pin_session, sync_sessions,
};
use http_service::http_service::user_service::dto::friend_request_info_dto::FriendRequestInfoDTO;
use http_service::http_service::user_service::service::friend_service::{
    FRIEND_ACCEPT_DEFAULT_MSG, FRIEND_REQUEST_DEFAULT_MSG, add_friend, delete_friend_service,
    process_friend,
};
use http_service::middleware::TraceIdMiddleware;
use http_service::state::AppState;
use http_service::utils::auth_middleware::auth_middleware;
use rbatis::RBatis;
use rbatis::rbdc::Uuid as RbatisUuid;
use rbatis::rbdc::db::ConnectOptions;
use rbatis::rbdc::pool::{ConnectionManager, Pool as RbatisPool};
use rbdc_pg::PgDriver;
use rbdc_pg::options::PgConnectOptions;
use rbdc_pool_fast::FastPool;
use s3_service::S3Client;
use s3_service::config::S3Config;
use serde_json::Value as JsonValue;
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt};
use uuid::Uuid;

/// 独立测试库名（与 entity 的 `only_talk_test` 区分，避免互相干扰）
const TEST_DATABASE_NAME: &str = "only_talk_http_test";

// 种子用户（直接写入测试库，用于登录等场景）
const SEED_ACCOUNT: &str = "seed_user_1";
const SEED_USERNAME: &str = "Seed User One";
const SEED_EMAIL: &str = "seed_user_1@example.com";
const SEED_PASSWORD: &str = "SeedPass12345678";

// 设备指纹（64 位 hex，模拟客户端生成的设备指纹）
const SEED_DEVICE_FP: &str = "a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90";
const WRONG_DEVICE_FP: &str = "feedfacefeedfacefeedfacefeedfacefeedfacefeedfacefeedfacefeedface";

// 新注册用户（走两步注册: step1 创建占位 -> complete_profile 补全）
const NEW_ACCOUNT: &str = "new_user_1";
const NEW_USERNAME: &str = "New User One";
const NEW_EMAIL: &str = "new_user_1@example.com";
const NEW_PASSWORD: &str = "NewUserPass123456";

// 验证码错误的用户（step1 应失败, 不创建占位）
const WRONG_EMAIL: &str = "wrong_user_1@example.com";

// 用于 complete_profile 账号冲突场景的邮箱（占位创建成功后用已存在账号补全应失败）
const CONFLICT_EMAIL: &str = "conflict_user_1@example.com";

// 用于会话 token 过期后重新继续注册场景的邮箱（占位用户允许重新走 step1 继续注册）
const RESUME_EMAIL: &str = "resume_user_1@example.com";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "需要本地 PostgreSQL、Redis 与仓库根目录 .env"]
async fn http_service_user_api_integration() -> Result<()> {
    init_tracing();
    dotenvy::dotenv().ok();
    // cargo 运行集成测试时 cwd 是包目录(crates/http_service)，dotenv() 可能找不到仓库根目录的 .env，
    // 这里用绝对路径兜底加载（.env 不在时返回 Err，可忽略）
    let _ = dotenvy::from_path(concat!(env!("CARGO_MANIFEST_DIR"), "/../../.env"));
    // 测试专用连接一律直接读 .env（不落入 app_config.toml）：
    // 管理员连接优先 TEST_DATABASE_URL, 缺省回退 DATABASE_URL; 测试 Redis 用 TEST_REDIS_URL。
    let admin_url =
        std::env::var("TEST_DATABASE_URL").or_else(|_| std::env::var("DATABASE_URL")).map_err(
            |_| anyhow!("未找到 TEST_DATABASE_URL / DATABASE_URL，请确认仓库根目录存在 .env 文件"),
        )?;
    let admin_url = common::substitute_env_vars(admin_url);
    info!("管理员数据库连接: {}", mask_url(&admin_url));
    let test_redis_url = std::env::var("TEST_REDIS_URL").map_err(|_| {
        anyhow!("未找到 TEST_REDIS_URL，请在仓库根目录 .env 中配置（建议独立 DB index，如 redis://127.0.0.1:6379/15）")
    })?;
    info!("测试 Redis 连接: {}", mask_url(&test_redis_url));

    let admin = build_db_pool(&admin_url).await?;
    info!("已连接管理员数据库");
    recreate_database(&admin, TEST_DATABASE_NAME).await?;
    let redis_pool = build_redis_pool(&test_redis_url)?;
    verify_redis_pool(&redis_pool).await?;

    // 测试逻辑与资源创建。用 catch_unwind 包裹，确保断言失败(panic)时
    // 也能走到下方的清理逻辑（删库 + 清空 Redis），不留下脏数据。
    let outcome = std::panic::AssertUnwindSafe(async {
        let test_url = test_database_url(&admin_url, TEST_DATABASE_NAME)?;
        let test_rb = build_db_pool(&test_url).await?;
        info!("已连接测试数据库 {}", TEST_DATABASE_NAME);

        entity::ddl::apply_all_ddl(&test_rb).await.context("应用 DDL 失败")?;
        for table in ["basic_user", "user_info", "friend_link", "group_info", "email_sso", "report"]
        {
            if !table_exists(&test_rb, table).await? {
                return Err(anyhow!("表 {} 未创建", table));
            }
        }
        info!("关键表全部创建成功");

        // 测试前清空测试 Redis，保证可重复执行
        flush_test_redis(&redis_pool).await?;

        // 种子用户：直接写入测试库（密码用与业务一致的 Argon2 哈希）
        let seed_uuid = Uuid::now_v7();
        let seed_uuid_rbdc: RbatisUuid = seed_uuid.to_string().parse().context("解析 UUID 失败")?;
        let hashed = hash_password(SEED_PASSWORD).context("生成密码哈希失败")?;
        BasicUser::insert(
            &test_rb,
            &BasicUser {
                uuid: Some(seed_uuid_rbdc.clone()),
                username: Some(SEED_USERNAME.to_string()),
                account: Some(SEED_ACCOUNT.to_string()),
                icon: None,
                info: Some(String::new()),
                password: Some(hashed),
                registration_status: Some(1),
                user_type: Some(0),
            },
        )
        .await
        .context("写入种子用户失败")?;
        let now = get_now_time_stamp_as_millis().context("获取时间戳失败")?;
        UserInfo::insert(
            &test_rb,
            &UserInfo {
                uuid: Some(seed_uuid_rbdc.clone()),
                gender: Some(0),
                age: Some(0),
                birthday: Some(0),
                note: Some("集成测试种子用户".to_string()),
                created_at: Some(now),
                updated_at: Some(now),
                phone: None,
                email: None,
                address: None,
                status: Some(0),
            },
        )
        .await
        .context("写入种子用户详情失败")?;
        EmailSso::insert(
            &test_rb,
            &EmailSso {
                uuid: Some(seed_uuid_rbdc.clone()),
                email: Some(SEED_EMAIL.to_string()),
                email_normalized: Some(SEED_EMAIL.to_lowercase()),
                verified: Some(true),
                verified_at: Some(now),
                verify_code_issued_at: Some(now),
                is_primary: Some(true),
                status: Some(1),
                last_login_at: None,
                last_login_ip: None,
                login_count: Some(0),
                fail_count: Some(0),
                locked_until: None,
                created_at: Some(now),
                updated_at: Some(now),
                deleted_at: None,
            },
        )
        .await
        .context("写入种子用户邮箱渠道失败")?;
        info!("种子用户已写入: {}", SEED_ACCOUNT);

        let state = AppState {
            core: CoreState { db: test_rb.clone(), redis: redis_pool.clone() },
            s3: Arc::new(
                S3Client::new(S3Config::default_minio()).await.context("初始化 S3 客户端失败")?,
            ),
            email: Arc::new(
                EmailManager::new(EmailServiceConfig::default())
                    .context("初始化 EmailManager 失败")?,
            ),
        };

        let app = test::init_service(
            App::new()
                .wrap(TraceIdMiddleware)
                .wrap(from_fn(auth_middleware))
                .app_data(web::Data::new(state))
                .configure(configure_routes),
        )
        .await;
        info!("Actix 测试服务已启动");

        // ===== 1. 鉴权：无 token 访问受保护接口，中间件应直接拒绝（返回 Err）=====
        let result =
            test::try_call_service(&app, test::TestRequest::post().uri("/user/me").to_request())
                .await;
        assert!(
            result.is_err(),
            "无 token 访问 /user/me 应被鉴权中间件拒绝（Err），实际: {:?}",
            result.as_ref().map(|r| r.status())
        );

        // ===== 2. 种子用户登录 =====
        let sign_in_body = json_obj(&[
            ("account", SEED_ACCOUNT),
            ("password", SEED_PASSWORD),
            ("platform", "PC"),
            ("device_fingerprint", SEED_DEVICE_FP),
        ]);
        let (status, json) = post_json(&app, "/user/sign_in", Some(&sign_in_body), None).await;
        assert_eq!(status, StatusCode::OK, "登录应成功: {json}");
        assert_eq!(json["code"], 200, "登录响应 code 应为 200: {json}");
        let access_token =
            json["data"]["access_token"].as_str().context("登录响应缺少 access_token")?.to_string();
        let refresh_token = json["data"]["refresh_token"]
            .as_str()
            .context("登录响应缺少 refresh_token")?
            .to_string();
        info!("种子用户登录成功");

        // 2b. 缺少设备指纹的登录应被拒绝（必填校验）
        let no_device_body =
            json_obj(&[("account", SEED_ACCOUNT), ("password", SEED_PASSWORD), ("platform", "PC")]);
        let (status, json) = post_json(&app, "/user/sign_in", Some(&no_device_body), None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "缺少设备指纹登录应被拒绝: {json}");

        // ===== 3. 鉴权后的用户信息查询 =====
        let (status, json) = post_json(&app, "/user/me", None, Some(&access_token)).await;
        assert_eq!(status, StatusCode::OK, "查询 /user/me 应成功: {json}");
        assert_eq!(json["code"], 200, "me 响应 code 应为 200: {json}");
        assert_eq!(json["data"]["account"], SEED_ACCOUNT, "me 应返回种子用户信息: {json}");

        let (status, json) = post_json(
            &app,
            &format!("/user/get_user_by_account/{}", SEED_ACCOUNT),
            None,
            Some(&access_token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "按账号查询应成功: {json}");
        assert_eq!(json["data"]["account"], SEED_ACCOUNT, "按账号查询返回错误用户: {json}");

        let (status, json) = post_json(
            &app,
            &format!("/user/get_uuid_by_account/{}", SEED_ACCOUNT),
            None,
            Some(&access_token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "按账号查询 uuid 应成功: {json}");
        assert_eq!(json["data"], seed_uuid.to_string(), "按账号查询 uuid 结果不一致: {json}");

        let (status, json) =
            get_json(&app, &format!("/user/get_user_by_uuid/{}", seed_uuid), Some(&access_token))
                .await;
        assert_eq!(status, StatusCode::OK, "按 uuid 查询应成功: {json}");
        assert_eq!(json["data"]["account"], SEED_ACCOUNT, "按 uuid 查询返回错误用户: {json}");

        // 账号存在性判断（请求体为纯文本账号）
        let resp = test::call_service(
            &app,
            test::TestRequest::post()
                .uri("/user/get_exit_user_flag/is_exit")
                .set_payload(SEED_ACCOUNT.to_string())
                .insert_header(("Authorization", access_token.as_str()))
                .to_request(),
        )
        .await;
        let body = test::read_body(resp).await;
        let text = std::str::from_utf8(&body).context("get_exit_user_flag 响应应为 UTF-8")?;
        assert_eq!(text, "true", "种子用户应判定为存在: {}", text);
        info!("鉴权用户接口全部通过");

        // ===== 4. refresh_token 换取新 access_token =====
        let refresh_body = json_obj(&[
            ("refresh_token", refresh_token.as_str()),
            ("device_fingerprint", SEED_DEVICE_FP),
        ]);
        let (status, json) =
            post_json(&app, "/user/refresh_token", Some(&refresh_body), None).await;
        assert_eq!(status, StatusCode::OK, "刷新 token 应成功: {json}");
        assert_eq!(json["code"], 200, "refresh_token 响应 code 应为 200: {json}");
        assert!(
            json["data"]["access_token"].as_str().is_some(),
            "refresh_token 应返回新的 access_token: {json}"
        );
        info!("refresh_token 刷新成功");

        // 4b. 设备指纹不匹配的 refresh_token 请求应被拒绝
        let wrong_refresh_body = json_obj(&[
            ("refresh_token", refresh_token.as_str()),
            ("device_fingerprint", WRONG_DEVICE_FP),
        ]);
        let (status, json) =
            post_json(&app, "/user/refresh_token", Some(&wrong_refresh_body), None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "设备指纹不匹配应被拒绝: {json}");
        assert_ne!(json["code"], 200, "设备指纹不匹配不应返回 200: {json}");
        info!("设备指纹不匹配的刷新被拒绝");

        // ===== 5. 两步注册流程 =====
        let mut conn = redis_pool.get().await.context("获取 Redis 连接失败")?;

        // 5.1 step1 验证码错误 -> 400, 不创建占位
        // 为 WRONG_EMAIL 预置一个错误验证码, 提交不匹配的验证码应失败
        let wrong_code_key = format!("{}{}", EMAIL_VERIFY_CODE, WRONG_EMAIL).to_uppercase();
        let _: () = conn.set_ex(&wrong_code_key, "000000", 300).await.context("写入验证码失败")?;
        let step1_wrong_body = json_obj(&[("email", WRONG_EMAIL), ("verification_code", "999999")]);
        let (status, json) =
            post_json(&app, "/user/sign_up_step1", Some(&step1_wrong_body), None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "验证码错误应返回 400: {json}");

        // 5.2 step1 成功: 为 NEW_EMAIL 预置正确验证码, 创建占位用户, 返回 reg_token + uuid
        let code_key = format!("{}{}", EMAIL_VERIFY_CODE, NEW_EMAIL).to_uppercase();
        let _: () = conn.set_ex(&code_key, "123456", 300).await.context("写入注册验证码失败")?;
        let step1_body = json_obj(&[("email", NEW_EMAIL), ("verification_code", "123456")]);
        let (status, json) = post_json(&app, "/user/sign_up_step1", Some(&step1_body), None).await;
        assert_eq!(status, StatusCode::OK, "step1 应成功: {json}");
        assert_eq!(json["code"], 200, "step1 响应 code 应为 200: {json}");
        let reg_token = json["data"]["reg_token"].as_str().context("缺少 reg_token")?.to_string();
        let reg_uuid = json["data"]["uuid"].as_str().context("缺少 uuid")?.to_string();
        let reg_uuid_rbdc: RbatisUuid = reg_uuid.parse().context("解析占位 uuid 失败")?;
        info!("step1 创建占位用户: uuid={}", reg_uuid);

        // 占位用户已写入 basic_user, registration_status=0, account 以 u_ 开头
        let placeholder = BasicUser::select_by_uuid(&test_rb, &reg_uuid_rbdc)
            .await
            .context("查询占位用户失败")?
            .expect("占位用户应存在");
        assert_eq!(placeholder.registration_status, Some(0), "占位用户 registration_status 应为 0");
        assert!(
            placeholder.account.as_deref().map(|a| a.starts_with("u_")).unwrap_or(false),
            "占位账号应以 u_ 开头: {:?}",
            placeholder.account
        );

        // email_sso 已创建并指向该占位用户
        let email_sso = EmailSso::select_by_uuid(&test_rb, &reg_uuid_rbdc)
            .await
            .context("查询 email_sso 失败")?
            .expect("占位用户 email_sso 应存在");
        assert_eq!(email_sso.email_normalized.as_deref(), Some(NEW_EMAIL), "email_sso 邮箱应一致");

        // 注册会话 token 已写入 Redis 并映射到占位用户 uuid
        let token_key = format!("{}{}", REGISTER_SESSION_TOKEN, reg_token).to_uppercase();
        let stored_uuid: Option<String> = conn.get(&token_key).await.context("读取注册会话失败")?;
        assert_eq!(stored_uuid.as_deref(), Some(reg_uuid.as_str()), "注册会话应映射到占位 uuid");

        // 5.3 占位账号登录被拦截
        let placeholder_account = placeholder.account.clone().expect("占位账号");
        let placeholder_login = json_obj(&[
            ("account", placeholder_account.as_str()),
            ("password", "SomeValidPass123456"),
            ("platform", "PC"),
            ("device_fingerprint", SEED_DEVICE_FP),
        ]);
        let (status, json) = post_json(&app, "/user/sign_in", Some(&placeholder_login), None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "占位用户登录应被拦截: {json}");

        // 5.4 step1 重复同一邮箱(占位未完成) -> 400
        let (status, json) = post_json(&app, "/user/sign_up_step1", Some(&step1_body), None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "重复 step1 同一邮箱应返回 400: {json}");

        // 5.4b complete_profile 邮箱不匹配(防 token 冒用/重放) -> 400, token 未被消费
        let mismatched_body = json_obj(&[
            ("reg_token", reg_token.as_str()),
            ("email", WRONG_EMAIL),
            ("account", "other_acct_123"),
            ("password", NEW_PASSWORD),
            ("username", "Other Name"),
        ]);
        let (status, json) =
            post_json(&app, "/user/complete_profile", Some(&mismatched_body), None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "邮箱不匹配应返回 400: {json}");
        let still_there: Option<String> = conn.get(&token_key).await.context("读取注册会话失败")?;
        assert!(still_there.is_some(), "邮箱不匹配时 token 不应被消费");

        // 5.5 complete_profile 成功: 用 reg_token 补全账号/用户名/密码
        let complete_body = json_obj(&[
            ("reg_token", reg_token.as_str()),
            ("email", NEW_EMAIL),
            ("account", NEW_ACCOUNT),
            ("password", NEW_PASSWORD),
            ("username", NEW_USERNAME),
        ]);
        let (status, json) =
            post_json(&app, "/user/complete_profile", Some(&complete_body), None).await;
        assert_eq!(status, StatusCode::OK, "complete_profile 应成功: {json}");
        assert_eq!(json["code"], 204, "complete_profile 成功应返回 code 204: {json}");
        info!("补全资料成功: account={}", NEW_ACCOUNT);

        // 补全后 registration_status=1, account 已改为自定义账号
        let completed = BasicUser::select_by_uuid(&test_rb, &reg_uuid_rbdc)
            .await
            .context("查询补全用户失败")?
            .expect("补全用户应存在");
        assert_eq!(completed.registration_status, Some(1), "补全后 registration_status 应为 1");
        assert_eq!(completed.account.as_deref(), Some(NEW_ACCOUNT), "补全后 account 应更新");
        assert_eq!(completed.username.as_deref(), Some(NEW_USERNAME), "补全后 username 应更新");

        // 注册会话 token 已消费(Redis 中已删除)
        let stored_uuid: Option<String> = conn.get(&token_key).await.context("读取注册会话失败")?;
        assert!(stored_uuid.is_none(), "注册会话应已被消费");

        // 5.6 旧占位账号失效, 新账号可登录
        let (status, json) = post_json(&app, "/user/sign_in", Some(&placeholder_login), None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "旧占位账号登录应失败: {json}");
        let new_sign_in = json_obj(&[
            ("account", NEW_ACCOUNT),
            ("password", NEW_PASSWORD),
            ("platform", "PC"),
            ("device_fingerprint", SEED_DEVICE_FP),
        ]);
        let (status, json) = post_json(&app, "/user/sign_in", Some(&new_sign_in), None).await;
        assert_eq!(status, StatusCode::OK, "新账号登录应成功: {json}");
        assert_eq!(json["code"], 200, "新账号登录响应 code 应为 200: {json}");

        // 5.7 complete_profile token 重复使用 -> 400(token 已消费)
        let (status, json) =
            post_json(&app, "/user/complete_profile", Some(&complete_body), None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "重复使用 reg_token 应返回 400: {json}");

        // 5.8 complete_profile 无效 token -> 400
        let invalid_body = json_obj(&[
            ("reg_token", "nonexistent-token"),
            ("email", "some_other@example.com"),
            ("account", "some_acct_123"),
            ("password", NEW_PASSWORD),
            ("username", "Some Name"),
        ]);
        let (status, json) =
            post_json(&app, "/user/complete_profile", Some(&invalid_body), None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "无效 reg_token 应返回 400: {json}");

        // 5.9 complete_profile 账号冲突 -> 400
        // 先用 CONFLICT_EMAIL 创建另一个占位用户
        let conflict_code_key = format!("{}{}", EMAIL_VERIFY_CODE, CONFLICT_EMAIL).to_uppercase();
        let _: () =
            conn.set_ex(&conflict_code_key, "123456", 300).await.context("写入验证码失败")?;
        let conflict_step1 =
            json_obj(&[("email", CONFLICT_EMAIL), ("verification_code", "123456")]);
        let (status, json) =
            post_json(&app, "/user/sign_up_step1", Some(&conflict_step1), None).await;
        assert_eq!(status, StatusCode::OK, "CONFLICT_EMAIL step1 应成功: {json}");
        let conflict_token =
            json["data"]["reg_token"].as_str().context("缺少 reg_token")?.to_string();
        // 用已存在的种子账号 SEED_ACCOUNT 补全应冲突
        let conflict_body = json_obj(&[
            ("reg_token", conflict_token.as_str()),
            ("email", CONFLICT_EMAIL),
            ("account", SEED_ACCOUNT),
            ("password", NEW_PASSWORD),
            ("username", "Conflict User"),
        ]);
        let (status, json) =
            post_json(&app, "/user/complete_profile", Some(&conflict_body), None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "账号冲突应返回 400: {json}");

        // 5.10 会话 token 过期后重新继续注册: 占位用户(registration_status=0)允许重新走 step1
        // 5.10.1 step1 创建占位用户
        let resume_code_key = format!("{}{}", EMAIL_VERIFY_CODE, RESUME_EMAIL).to_uppercase();
        let _: () = conn.set_ex(&resume_code_key, "123456", 300).await.context("写入验证码失败")?;
        let resume_step1 = json_obj(&[("email", RESUME_EMAIL), ("verification_code", "123456")]);
        let (status, json) =
            post_json(&app, "/user/sign_up_step1", Some(&resume_step1), None).await;
        assert_eq!(status, StatusCode::OK, "RESUME_EMAIL step1 应成功: {json}");
        let resume_uuid = json["data"]["uuid"].as_str().context("缺少 uuid")?.to_string();

        // 5.10.2 模拟会话 token 过期: 删除 Redis 中的注册会话 token
        let resume_token_key = format!(
            "{}{}",
            REGISTER_SESSION_TOKEN,
            json["data"]["reg_token"].as_str().context("缺少 reg_token")?
        )
        .to_uppercase();
        let _: () = conn.del(&resume_token_key).await.context("删除注册会话失败")?;

        // 5.10.3 重新获取验证码后再次 step1 应成功, 且复用同一占位用户(uuid 不变)
        let _: () =
            conn.set_ex(&resume_code_key, "654321", 300).await.context("重新写入验证码失败")?;
        let resume_step1_2 = json_obj(&[("email", RESUME_EMAIL), ("verification_code", "654321")]);
        let (status, json) =
            post_json(&app, "/user/sign_up_step1", Some(&resume_step1_2), None).await;
        assert_eq!(status, StatusCode::OK, "占位用户重新 step1 应成功: {json}");
        assert_eq!(
            json["data"]["uuid"].as_str(),
            Some(resume_uuid.as_str()),
            "占位用户应复用同一 uuid: {json}"
        );
        let resume_token2 =
            json["data"]["reg_token"].as_str().context("缺少 reg_token")?.to_string();

        // 5.10.4 用新 token 补全资料应成功
        let resume_complete = json_obj(&[
            ("reg_token", resume_token2.as_str()),
            ("email", RESUME_EMAIL),
            ("account", "resume_acct_1"),
            ("password", NEW_PASSWORD),
            ("username", "Resume User"),
        ]);
        let (status, json) =
            post_json(&app, "/user/complete_profile", Some(&resume_complete), None).await;
        assert_eq!(status, StatusCode::OK, "续注册 complete_profile 应成功: {json}");
        assert_eq!(json["code"], 204, "续注册 complete_profile 应返回 code 204: {json}");

        // 5.10.5 完成注册后, 该邮箱不可再被注册
        let (status, json) =
            post_json(&app, "/user/sign_up_step1", Some(&resume_step1_2), None).await;
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "已完成注册的邮箱再次 step1 应返回 400: {json}"
        );
        info!("两步注册流程全部通过");

        // ===== 6. 举报接口 =====
        // 6.1 种子举报目标：另一用户、群、动态、动态评论、广场资料
        let target_user_uuid = Uuid::now_v7();
        let target_user_uuid_rbdc: RbatisUuid =
            target_user_uuid.to_string().parse().context("解析目标用户 UUID 失败")?;
        // basic_user.password 为 NOT NULL, 目标用户同样需要哈希密码(与 sign_up_step1 占位用户同款做法)
        let target_hashed =
            hash_password(&target_user_uuid.to_string()).context("生成目标用户密码哈希失败")?;
        BasicUser::insert(
            &test_rb,
            &BasicUser {
                uuid: Some(target_user_uuid_rbdc.clone()),
                username: Some("Report Target".to_string()),
                account: Some("report_target_1".to_string()),
                icon: None,
                info: Some(String::new()),
                password: Some(target_hashed),
                registration_status: Some(1),
                user_type: Some(0),
            },
        )
        .await
        .context("写入举报目标用户失败")?;

        let group_uuid = Uuid::now_v7();
        let group_uuid_rbdc: RbatisUuid =
            group_uuid.to_string().parse().context("解析群 UUID 失败")?;
        GroupInfo::insert(
            &test_rb,
            &GroupInfo {
                id: None,
                group_uuid: Some(group_uuid_rbdc.clone()),
                group_name: Some("举报测试群".to_string()),
                avatar: None,
                owner_uuid: Some(seed_uuid_rbdc.clone()),
                description: None,
                max_members: Some(200),
                created_at: Some(now),
                updated_at: Some(now),
                status: Some(1),
            },
        )
        .await
        .context("写入举报目标群失败")?;

        let moment_uuid = Uuid::now_v7();
        let moment_uuid_rbdc: RbatisUuid =
            moment_uuid.to_string().parse().context("解析动态 UUID 失败")?;
        Moment::insert(
            &test_rb,
            &Moment {
                uuid: Some(moment_uuid_rbdc.clone()),
                author_uuid: Some(target_user_uuid_rbdc.clone()),
                content: Some("举报测试动态".to_string()),
                visibility: Some(0),
                is_del: Some(false),
                created_at: Some(now),
                updated_at: Some(now),
            },
        )
        .await
        .context("写入举报目标动态失败")?;

        let comment_id = Uuid::now_v7();
        let comment_id_rbdc: RbatisUuid =
            comment_id.to_string().parse().context("解析评论 UUID 失败")?;
        MomentComment::insert(
            &test_rb,
            &MomentComment {
                id: Some(comment_id_rbdc.clone()),
                moment_uuid: Some(moment_uuid_rbdc.clone()),
                author_uuid: Some(target_user_uuid_rbdc.clone()),
                content: Some("举报测试评论".to_string()),
                is_del: Some(false),
                created_at: Some(now),
            },
        )
        .await
        .context("写入举报目标评论失败")?;

        PlazaUserInfo::insert(
            &test_rb,
            &PlazaUserInfo {
                uuid: Some(target_user_uuid_rbdc.clone()),
                allow_discover: Some(true),
                motto: None,
                status: Some(0),
                created_at: Some(now),
                updated_at: Some(now),
            },
        )
        .await
        .context("写入举报目标广场资料失败")?;

        // 构造带数字 target_type 的举报请求体
        fn report_body(target_type: i16, target_uuid: &str, reason: &str) -> JsonValue {
            let mut map = serde_json::Map::new();
            map.insert("target_type".to_string(), JsonValue::from(target_type));
            map.insert("target_uuid".to_string(), JsonValue::String(target_uuid.to_string()));
            map.insert("reason".to_string(), JsonValue::String(reason.to_string()));
            JsonValue::Object(map)
        }

        // 6.2 五类举报均成功并落库
        let report_cases: [(&str, i16, String); 5] = [
            ("用户", 1, target_user_uuid.to_string()),
            ("群组", 2, group_uuid.to_string()),
            ("动态", 3, moment_uuid.to_string()),
            ("卡片匹配", 4, target_user_uuid.to_string()),
            ("动态评论", 5, comment_id.to_string()),
        ];
        for (label, target_type, target_uuid) in &report_cases {
            let body = report_body(*target_type, target_uuid, "涉嫌违规");
            let (status, json) =
                post_json(&app, "/report/create", Some(&body), Some(&access_token)).await;
            assert_eq!(status, StatusCode::OK, "举报{}应成功: {json}", label);
            assert_eq!(json["code"], 200, "举报{}响应 code 应为 200: {json}", label);
            assert_eq!(json["data"], true, "举报{}应返回 true: {json}", label);

            let target_uuid_rbdc: RbatisUuid =
                target_uuid.parse().context("解析举报目标 UUID 失败")?;
            let saved = Report::select_pending_by_reporter_target(
                &test_rb,
                &seed_uuid_rbdc,
                *target_type,
                &target_uuid_rbdc,
            )
            .await
            .context("查询举报记录失败")?;
            assert!(saved.is_some(), "举报{}记录应落库", label);
        }

        // 6.3 重复举报同一目标应被拒绝
        let dup_body = report_body(1, &target_user_uuid.to_string(), "再次举报");
        let (status, json) =
            post_json(&app, "/report/create", Some(&dup_body), Some(&access_token)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "重复举报应返回 400: {json}");

        // 6.4 举报自己应被拒绝
        let self_body = report_body(1, &seed_uuid.to_string(), "举报自己");
        let (status, json) =
            post_json(&app, "/report/create", Some(&self_body), Some(&access_token)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "举报自己应返回 400: {json}");

        // 6.5 空原因应被拒绝
        let empty_reason_body = report_body(3, &moment_uuid.to_string(), "   ");
        let (status, json) =
            post_json(&app, "/report/create", Some(&empty_reason_body), Some(&access_token)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "空原因举报应返回 400: {json}");

        // 6.6 目标不存在应被拒绝
        let missing_body = report_body(2, &Uuid::now_v7().to_string(), "目标不存在");
        let (status, json) =
            post_json(&app, "/report/create", Some(&missing_body), Some(&access_token)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "举报不存在目标应返回 400: {json}");
        info!("举报接口全部通过");

        // ===== 7. 任务08: 历史接口 session_uuid 化 + 旧接口 404 =====
        // 7.1 get_chat_record: 旧 path 参数(friend_uuid) 与新 Body 参数(session_uuid) 结果一致
        let peer_a = Uuid::new_v4();
        let peer_a_rbdc: RbatisUuid = peer_a.to_string().parse()?;
        let session_a: RbatisUuid = single_session_uuid(&seed_uuid, &peer_a).to_string().parse()?;
        for ts in [1_000i64, 2_000] {
            ChatMessageRecord::insert(
                &test_rb,
                &ChatMessageRecord {
                    id: None,
                    session_uuid: session_a.clone(),
                    nano_id: Some(format!("hist-a-{ts}")),
                    timestamp: Some(ts),
                    raw: b"hi".to_vec().into(),
                    text_type: Some(1),
                    send_user: peer_a_rbdc.clone(),
                    recv_user: seed_uuid_rbdc.clone(),
                },
            )
            .await
            .context("插入历史测试消息失败")?;
        }
        let old_body = {
            let mut m = serde_json::Map::new();
            m.insert("page_num".into(), JsonValue::from(0));
            m.insert("page_size".into(), JsonValue::from(10));
            JsonValue::Object(m)
        };
        let (status, old_json) = post_json(
            &app,
            &format!("/msg/get_chat_record/{}", peer_a),
            Some(&old_body),
            Some(&access_token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "旧参数查历史应成功: {old_json}");
        let new_body = {
            let mut m = serde_json::Map::new();
            m.insert("session_uuid".into(), JsonValue::String(session_a.to_string()));
            m.insert("page_num".into(), JsonValue::from(0));
            m.insert("page_size".into(), JsonValue::from(10));
            JsonValue::Object(m)
        };
        let (status, new_json) = post_json(
            &app,
            &format!("/msg/get_chat_record/{}", Uuid::now_v7()),
            Some(&new_body),
            Some(&access_token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "新参数查历史应成功: {new_json}");
        let old_nanos: Vec<String> = old_json["data"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v["nano_id"].as_str().map(str::to_string)).collect())
            .unwrap_or_default();
        let new_nanos: Vec<String> = new_json["data"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v["nano_id"].as_str().map(str::to_string)).collect())
            .unwrap_or_default();
        assert_eq!(
            old_nanos,
            vec!["hist-a-1000", "hist-a-2000"],
            "旧参数应返回该会话消息: {old_json}"
        );
        assert_eq!(old_nanos, new_nanos, "新参数与旧参数派生应返回同一会话: {new_json}");

        // 7.2 旧接口 404 回归(任务08 已删除路由)
        for uri in ["/msg/get_unread_chat_record", "/msg/add_read_chat_record"] {
            let (status, _) = post_json(&app, uri, None, Some(&access_token)).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{uri} 应 404(路由已删)");
        }
        info!("任务08 历史接口 session_uuid 化 / 旧接口 404 通过");
        // 7.3 /session/read 钳制到 synced_id + /session/synced 推进
        let session_d: RbatisUuid = Uuid::new_v4().to_string().parse()?;
        UserSession::upsert(
            &test_rb,
            &UserSession {
                id: None,
                user_uuid: seed_uuid_rbdc.clone(),
                session_uuid: session_d.clone(),
                session_type: Some(SESSION_TYPE_SINGLE),
                peer_uuid: None,
                last_read_id: None,
                synced_id: Some(100),
                pinned: None,
                muted: None,
                deleted_at: None,
                created_at: None,
                updated_at: None,
            },
        )
        .await
        .context("upsert 钳制会话失败")?;
        let read_req = |id: i64| {
            let mut m = serde_json::Map::new();
            m.insert("session_uuid".into(), JsonValue::String(session_d.to_string()));
            m.insert("session_type".into(), JsonValue::from(1));
            m.insert("last_read_id".into(), JsonValue::from(id));
            let mut outer = serde_json::Map::new();
            outer.insert("reads".into(), JsonValue::Array(vec![JsonValue::Object(m)]));
            JsonValue::Object(outer)
        };
        let (status, json) =
            post_json(&app, "/session/read", Some(&read_req(150)), Some(&access_token)).await;
        assert_eq!(status, StatusCode::OK, "/session/read 应成功: {json}");
        let us_d = UserSession::select_by_map(
            &test_rb,
            rbs::value! {"user_uuid": &seed_uuid_rbdc, "session_uuid": &session_d},
        )
        .await
        .context("查询钳制 user_session 失败")?;
        assert_eq!(us_d[0].last_read_id, Some(100), "上报 150 应被钳制到 synced_id=100");

        // 推进 synced 到 200 后再报 150 → 前进到 150
        let synced_req = {
            let mut m = serde_json::Map::new();
            m.insert("session_uuid".into(), JsonValue::String(session_d.to_string()));
            m.insert("synced_id".into(), JsonValue::from(200i64));
            let mut outer = serde_json::Map::new();
            outer.insert("sessions".into(), JsonValue::Array(vec![JsonValue::Object(m)]));
            JsonValue::Object(outer)
        };
        let (status, json) =
            post_json(&app, "/session/synced", Some(&synced_req), Some(&access_token)).await;
        assert_eq!(status, StatusCode::OK, "/session/synced 应成功: {json}");
        let (status, _json) =
            post_json(&app, "/session/read", Some(&read_req(150)), Some(&access_token)).await;
        assert_eq!(status, StatusCode::OK, "二次 /session/read 应成功");
        let us_d2 = UserSession::select_by_map(
            &test_rb,
            rbs::value! {"user_uuid": &seed_uuid_rbdc, "session_uuid": &session_d},
        )
        .await
        .context("二次查询钳制 user_session 失败")?;
        assert_eq!(us_d2[0].synced_id, Some(200), "synced 应推进到 200");
        assert_eq!(us_d2[0].last_read_id, Some(150), "synced 放宽后已读应前进到 150");
        info!("任务04 已读桥接 / 缺陷A / session 接口全部通过");

        // ===== 8. 任务04b: 好友通过即建会话 =====
        // 直接调 service 函数(免二次登录), 新建临时用户
        let (user_a, user_a_rbdc) = seed_friend_user(&test_rb, "a").await?;
        let (user_b, user_b_rbdc) = seed_friend_user(&test_rb, "b").await?;
        let session_ab: RbatisUuid = single_session_uuid(&user_a, &user_b).to_string().parse()?;

        // 8.1 接受好友 → session + 双方 user_session + 两条前置消息
        let request = add_friend(
            &test_rb,
            friend_dto(&user_a, &user_b, Some("我是老王，群里的"), None, None),
        )
        .await
        .context("发起好友申请失败")?;
        assert_eq!(request.accept_status, Some(0), "新申请应为待处理");
        let accepted = process_friend(
            &test_rb,
            friend_dto(&user_a, &user_b, None, Some("你好，通过了"), Some(1)),
        )
        .await
        .context("通过好友申请失败")?;
        assert_eq!(accepted.accept_status, Some(1), "接受后请求状态应为 1");

        let sessions = Session::select_by_map(&test_rb, rbs::value! {"session_uuid": &session_ab})
            .await
            .context("查询 session 失败")?;
        assert_eq!(sessions.len(), 1, "接受好友应创建 1 条 session");
        assert_eq!(sessions[0].session_type, Some(SESSION_TYPE_SINGLE));
        assert_eq!(sessions[0].last_preview.as_deref(), Some("你好，通过了"), "摘要应为通过留言");
        let last_id = sessions[0].last_message_id.context("session.last_message_id 应非空")?;

        let us_a = UserSession::select_by_map(
            &test_rb,
            rbs::value! {"user_uuid": &user_a_rbdc, "session_uuid": &session_ab},
        )
        .await
        .context("查询 A user_session 失败")?;
        assert_eq!(us_a.len(), 1, "A 应有 1 条 user_session");
        assert_eq!(us_a[0].peer_uuid, Some(user_b_rbdc.clone()), "A 的 peer 应为 B");
        assert_eq!(us_a[0].last_read_id, Some(0), "A 游标应初始化为 0(upsert COALESCE)");
        assert_eq!(us_a[0].synced_id, Some(0), "A synced 应为 0");
        let us_b = UserSession::select_by_map(
            &test_rb,
            rbs::value! {"user_uuid": &user_b_rbdc, "session_uuid": &session_ab},
        )
        .await
        .context("查询 B user_session 失败")?;
        assert_eq!(us_b.len(), 1, "B 应有 1 条 user_session");
        assert_eq!(us_b[0].peer_uuid, Some(user_a_rbdc.clone()), "B 的 peer 应为 A");

        let msgs = select_session_messages(&test_rb, &session_ab).await?;
        assert_eq!(msgs.len(), 2, "接受好友应写 2 条前置消息");
        assert_eq!(msgs[0].send_user, user_a_rbdc.clone(), "第 1 条应为 A→B");
        assert_eq!(msgs[0].recv_user, user_b_rbdc.clone());
        assert_eq!(msg_text(&msgs[0])?, "我是老王，群里的");
        assert_eq!(msgs[1].send_user, user_b_rbdc.clone(), "第 2 条应为 B→A");
        assert_eq!(msgs[1].recv_user, user_a_rbdc.clone());
        assert_eq!(msg_text(&msgs[1])?, "你好，通过了");
        assert_eq!(msgs[1].id, Some(last_id), "摘要应指向最新一条(B 条)");
        assert_eq!(msgs[0].text_type, Some(0));
        assert_eq!(msgs[1].text_type, Some(0));

        // 8.2 双向投递(经 /session/sync): 会话内双向消息都同步, 未读归属由客户端按 recv_user 计算
        let has_pair = |msgs: &[(String, String)], send: &str, recv: &str| {
            msgs.iter().any(|(s, r)| s == send && r == recv)
        };
        let sync_a = sync_sessions(
            &test_rb,
            Some(user_a.to_string()),
            sync_req(Some(vec![sync_session_req(&session_ab)]), None),
        )
        .await
        .context("A 同步失败")?;
        let msgs_a: Vec<(String, String)> = sync_a
            .sessions
            .iter()
            .flat_map(|s| {
                s.messages
                    .iter()
                    .map(|m| (m.send_user.clone(), m.recv_user.clone()))
                    .collect::<Vec<_>>()
            })
            .collect();
        assert!(
            has_pair(&msgs_a, &user_b.to_string(), &user_a.to_string()),
            "A 同步应含 B→A 的通过提示: {msgs_a:?}"
        );
        let sync_b = sync_sessions(
            &test_rb,
            Some(user_b.to_string()),
            sync_req(Some(vec![sync_session_req(&session_ab)]), None),
        )
        .await
        .context("B 同步失败")?;
        let msgs_b: Vec<(String, String)> = sync_b
            .sessions
            .iter()
            .flat_map(|s| {
                s.messages
                    .iter()
                    .map(|m| (m.send_user.clone(), m.recv_user.clone()))
                    .collect::<Vec<_>>()
            })
            .collect();
        assert!(
            has_pair(&msgs_b, &user_a.to_string(), &user_b.to_string()),
            "B 同步应含 A→B 的申请留言: {msgs_b:?}"
        );
        info!("任务04b 好友通过即建会话 / 双向投递 通过");

        // 8.3 删好友后带道歉重加 → 道歉留言进入会话, 消息数=4
        delete_friend_service(&test_rb, Some(user_a.to_string()), user_b.to_string())
            .await
            .context("删除好友失败")?;
        add_friend(
            &test_rb,
            friend_dto(&user_a, &user_b, Some("对不起，之前是我不对"), None, None),
        )
        .await
        .context("重加好友申请失败")?;
        process_friend(&test_rb, friend_dto(&user_a, &user_b, None, None, Some(1)))
            .await
            .context("重加好友通过失败")?;
        let msgs = select_session_messages(&test_rb, &session_ab).await?;
        assert_eq!(msgs.len(), 4, "重加后应为 4 条消息(旧 2 + 新 2)");
        assert_eq!(msgs[2].send_user, user_a_rbdc.clone(), "第 3 条应为道歉 A→B");
        assert_eq!(msg_text(&msgs[2])?, "对不起，之前是我不对", "重加应保留本次道歉留言");
        assert_eq!(msg_text(&msgs[3])?, FRIEND_ACCEPT_DEFAULT_MSG, "第 4 条应为通过模板");

        // 8.4 事务原子性(缺陷R): FriendLink 主键冲突 → 全部回滚
        let (user_c, user_c_rbdc) = seed_friend_user(&test_rb, "c").await?;
        let (user_d, user_d_rbdc) = seed_friend_user(&test_rb, "d").await?;
        let session_cd: RbatisUuid = single_session_uuid(&user_c, &user_d).to_string().parse()?;
        let req_cd =
            add_friend(&test_rb, friend_dto(&user_c, &user_d, Some("hi"), None, None)).await?;
        let req_cd_uuid = req_cd.uuid.clone().context("请求 uuid 缺失")?;
        // 预插一条同 uuid 但不同用户对的 friend_link → 接受时 FriendLink::insert 主键冲突
        FriendLink::insert(
            &test_rb,
            &FriendLink {
                uuid: Some(req_cd_uuid.clone()),
                request_user: Some(RbatisUuid::new()),
                accept_user: Some(RbatisUuid::new()),
                is_del: Some(false),
                created_at: Some(0),
                updated_at: Some(0),
                version: Some(0),
            },
        )
        .await
        .context("预插冲突 friend_link 失败")?;
        let failed =
            process_friend(&test_rb, friend_dto(&user_c, &user_d, None, None, Some(1))).await;
        assert!(failed.is_err(), "FriendLink 主键冲突应导致接受失败");
        // 申请状态必须回滚为待处理(缺陷R: 语句在同一事务)
        let reqs = FriendRequestInfo::select_by_uuid(&test_rb, &user_c_rbdc, &user_d_rbdc)
            .await
            .context("查询好友请求失败")?;
        let target = reqs
            .iter()
            .find(|r| r.uuid == Some(req_cd_uuid.clone()))
            .context("未找到目标好友请求")?;
        assert_eq!(target.accept_status, Some(0), "失败后申请状态应回滚为待处理");
        assert!(
            Session::select_by_map(&test_rb, rbs::value! {"session_uuid": &session_cd})
                .await?
                .is_empty(),
            "失败后不应残留 session 行"
        );
        assert!(
            UserSession::select_by_map(&test_rb, rbs::value! {"session_uuid": &session_cd})
                .await?
                .is_empty(),
            "失败后不应残留 user_session 行"
        );
        assert!(
            select_session_messages(&test_rb, &session_cd).await?.is_empty(),
            "失败后不应残留消息"
        );

        // 8.5 申请/通过均无留言 → 两条默认文案
        let (user_e, _user_e_rbdc) = seed_friend_user(&test_rb, "e").await?;
        let (user_f, _user_f_rbdc) = seed_friend_user(&test_rb, "f").await?;
        let session_ef: RbatisUuid = single_session_uuid(&user_e, &user_f).to_string().parse()?;
        add_friend(&test_rb, friend_dto(&user_e, &user_f, None, None, None)).await?;
        process_friend(&test_rb, friend_dto(&user_e, &user_f, None, None, Some(1))).await?;
        let msgs = select_session_messages(&test_rb, &session_ef).await?;
        assert_eq!(msgs.len(), 2, "无留言也应恒写两条默认文案");
        assert_eq!(msg_text(&msgs[0])?, FRIEND_REQUEST_DEFAULT_MSG);
        assert_eq!(msg_text(&msgs[1])?, FRIEND_ACCEPT_DEFAULT_MSG);
        info!("任务04b 好友通过即建会话 全部通过");

        // ===== 9. 任务12: 离线同步 /session/sync(无状态窗口查询) =====
        let (sync_user, sync_user_rbdc) = seed_friend_user(&test_rb, "sync").await?;
        let sync_now = get_now_time_stamp_as_millis()?;
        let boundary_8d = sync_now - 8 * 24 * 3600 * 1000;

        // 9.1 首拉: after_id 缺省(=0) → 窗口内最旧 10 条(升序), has_more=true
        let peer_a = Uuid::now_v7();
        let peer_a_rbdc: RbatisUuid = peer_a.to_string().parse()?;
        let session_a: RbatisUuid = single_session_uuid(&sync_user, &peer_a).to_string().parse()?;
        let mut ids_a = Vec::new();
        for i in 0..25 {
            ids_a.push(
                insert_single_msg(
                    &test_rb,
                    &session_a,
                    &format!("sync-a-{i:02}"),
                    sync_now + i,
                    &peer_a_rbdc,
                    &sync_user_rbdc,
                )
                .await?,
            );
        }
        upsert_user_session(
            &test_rb,
            &sync_user_rbdc,
            &session_a,
            SESSION_TYPE_SINGLE,
            Some(&peer_a_rbdc),
            0,
        )
        .await?;
        let resp = sync_sessions(
            &test_rb,
            Some(sync_user.to_string()),
            sync_req(Some(vec![sync_session_req(&session_a)]), Some(10)),
        )
        .await
        .context("同步(9.1)失败")?;
        assert_eq!(resp.sessions.len(), 1, "应返回请求的会话: {resp:?}");
        assert_eq!(resp.sessions[0].messages.len(), 10);
        assert!(resp.sessions[0].has_more, "还有 15 条 → has_more=true");
        assert_eq!(resp.sessions[0].next_cursor, ids_a[9], "next_cursor=本批最大 id(末条)");
        assert_eq!(resp.sessions[0].messages[0].nano_id, "sync-a-00", "窗口内最早");
        assert_eq!(resp.sessions[0].messages[9].nano_id, "sync-a-09", "本批最新");
        let batch_ids: Vec<i64> = resp.sessions[0].messages.iter().map(|m| m.id).collect();
        assert!(batch_ids.windows(2).all(|w| w[0] < w[1]), "响应内 id 升序: {batch_ids:?}");

        // 9.2 续拉: after_id=上批最大 → 紧邻更新 10 条, 无重无漏; 翻至 has_more=false, 并集=全部 25
        let mut all_ids = batch_ids.clone();
        let mut after = resp.sessions[0].next_cursor;
        let mut last_has_more = resp.sessions[0].has_more;
        while last_has_more {
            let resp = sync_sessions(
                &test_rb,
                Some(sync_user.to_string()),
                sync_req(Some(vec![sync_session_req_after(&session_a, after)]), Some(10)),
            )
            .await
            .context("同步(9.2)失败")?;
            let page: Vec<i64> = resp.sessions[0].messages.iter().map(|m| m.id).collect();
            assert!(!page.is_empty(), "has_more=true 时不应空批");
            assert!(page.iter().all(|id| *id > after), "续拉应严格更新: {page:?} vs {after}");
            all_ids.extend(page.iter().copied());
            after = resp.sessions[0].next_cursor;
            last_has_more = resp.sessions[0].has_more;
        }
        all_ids.sort();
        all_ids.dedup();
        let mut expect_ids = ids_a.clone();
        expect_ids.sort();
        assert_eq!(all_ids, expect_ids, "翻页并集应等于全部 25 条且无重复");

        // 9.3 窗口过滤: 窗口外更旧消息不返回, has_more=false(7 天阈值即停止, 无截断标志)
        let peer_t = Uuid::now_v7();
        let peer_t_rbdc: RbatisUuid = peer_t.to_string().parse()?;
        let session_t: RbatisUuid = single_session_uuid(&sync_user, &peer_t).to_string().parse()?;
        insert_single_msg(
            &test_rb,
            &session_t,
            "sync-t-old",
            boundary_8d,
            &peer_t_rbdc,
            &sync_user_rbdc,
        )
        .await?;
        insert_single_msg(
            &test_rb,
            &session_t,
            "sync-t-0",
            sync_now,
            &peer_t_rbdc,
            &sync_user_rbdc,
        )
        .await?;
        upsert_user_session(
            &test_rb,
            &sync_user_rbdc,
            &session_t,
            SESSION_TYPE_SINGLE,
            Some(&peer_t_rbdc),
            0,
        )
        .await?;
        let resp = sync_sessions(
            &test_rb,
            Some(sync_user.to_string()),
            sync_req(Some(vec![sync_session_req(&session_t)]), Some(100)),
        )
        .await
        .context("同步(9.3)失败")?;
        assert_eq!(resp.sessions[0].messages.len(), 1, "窗口外消息应被过滤");
        assert!(!resp.sessions[0].has_more, "窗口内已取尽 → 追平, has_more=false");
        assert_eq!(resp.sessions[0].messages[0].nano_id, "sync-t-0");

        // 9.4 追平后空批: after_id 超前沿 → 空消息, has_more=false, next_cursor=after
        let peer_n = Uuid::now_v7();
        let peer_n_rbdc: RbatisUuid = peer_n.to_string().parse()?;
        let session_n: RbatisUuid = single_session_uuid(&sync_user, &peer_n).to_string().parse()?;
        let id_n = insert_single_msg(
            &test_rb,
            &session_n,
            "sync-n-0",
            sync_now,
            &peer_n_rbdc,
            &sync_user_rbdc,
        )
        .await?;
        upsert_user_session(
            &test_rb,
            &sync_user_rbdc,
            &session_n,
            SESSION_TYPE_SINGLE,
            Some(&peer_n_rbdc),
            0,
        )
        .await?;
        let resp = sync_sessions(
            &test_rb,
            Some(sync_user.to_string()),
            sync_req(Some(vec![sync_session_req_after(&session_n, id_n)]), Some(100)),
        )
        .await
        .context("同步(9.4)失败")?;
        assert!(resp.sessions[0].messages.is_empty(), "已追平 → 空批");
        assert!(!resp.sessions[0].has_more);
        assert_eq!(resp.sessions[0].next_cursor, id_n, "空批 next_cursor=请求的 after");

        // 9.5 归属校验: 不属于我的会话静默跳过, 非法 uuid 跳过, 其余正常
        let (other_user, other_rbdc) = seed_friend_user(&test_rb, "syncother").await?;
        let peer_o = Uuid::now_v7();
        let peer_o_rbdc: RbatisUuid = peer_o.to_string().parse()?;
        let session_o: RbatisUuid =
            single_session_uuid(&other_user, &peer_o).to_string().parse()?;
        insert_single_msg(&test_rb, &session_o, "sync-o-0", sync_now, &peer_o_rbdc, &other_rbdc)
            .await?;
        upsert_user_session(
            &test_rb,
            &other_rbdc,
            &session_o,
            SESSION_TYPE_SINGLE,
            Some(&peer_o_rbdc),
            0,
        )
        .await?;
        let resp = sync_sessions(
            &test_rb,
            Some(sync_user.to_string()),
            sync_req(
                Some(vec![
                    sync_session_req(&session_n),
                    sync_session_req(&session_o),
                    SyncSessionReq { session_uuid: "not-a-uuid".to_string(), after_id: None },
                ]),
                None,
            ),
        )
        .await
        .context("同步(9.5)失败")?;
        assert_eq!(resp.sessions.len(), 1, "只返回属于我的会话: {resp:?}");
        assert_eq!(resp.sessions[0].session_uuid, session_n.to_string());

        // 9.6 after_id 缺省 ≡ 0(从窗口内最早起)
        let resp_default = sync_sessions(
            &test_rb,
            Some(sync_user.to_string()),
            sync_req(Some(vec![sync_session_req(&session_a)]), Some(10)),
        )
        .await
        .context("同步(9.6a)失败")?;
        let resp_zero = sync_sessions(
            &test_rb,
            Some(sync_user.to_string()),
            sync_req(Some(vec![sync_session_req_after(&session_a, 0)]), Some(10)),
        )
        .await
        .context("同步(9.6b)失败")?;
        assert_eq!(
            resp_default.sessions[0].next_cursor, resp_zero.sessions[0].next_cursor,
            "缺省 after_id 应等价 0"
        );
        assert_eq!(
            resp_default.sessions[0].messages.len(),
            resp_zero.sessions[0].messages.len(),
            "缺省 after_id 应返回相同条数"
        );

        // 9.7 旧协议兼容: 多余 mode 字段被 serde 静默忽略
        let legacy: SyncRequestDTO = serde_json::from_str(
            r#"{"mode":"incremental","sessions":[{"session_uuid":"x"}],"limit":10}"#,
        )
        .context("旧协议请求应可反序列化(mode 被忽略)")?;
        assert_eq!(legacy.sessions.as_ref().map(|v| v.len()), Some(1));
        assert_eq!(
            legacy.sessions.as_ref().and_then(|v| v.first()).map(|s| s.after_id),
            Some(None),
            "after_id 缺省应为 None"
        );

        // 9.8 limit 服务端钳制(99999 → 200)
        let peer_h = Uuid::now_v7();
        let peer_h_rbdc: RbatisUuid = peer_h.to_string().parse()?;
        let session_h: RbatisUuid = single_session_uuid(&sync_user, &peer_h).to_string().parse()?;
        for i in 0..205 {
            insert_single_msg(
                &test_rb,
                &session_h,
                &format!("sync-h-{i}"),
                sync_now + i,
                &peer_h_rbdc,
                &sync_user_rbdc,
            )
            .await?;
        }
        upsert_user_session(
            &test_rb,
            &sync_user_rbdc,
            &session_h,
            SESSION_TYPE_SINGLE,
            Some(&peer_h_rbdc),
            0,
        )
        .await?;
        let resp = sync_sessions(
            &test_rb,
            Some(sync_user.to_string()),
            sync_req(Some(vec![sync_session_req(&session_h)]), Some(99999)),
        )
        .await
        .context("同步(9.8)失败")?;
        assert_eq!(resp.sessions[0].messages.len(), 200, "limit 应被钳制到 200");
        assert!(resp.sessions[0].has_more);

        // 9.9 空 sessions → 空响应(不再服务端筛会话)
        let resp = sync_sessions(&test_rb, Some(sync_user.to_string()), sync_req(None, None))
            .await
            .context("同步(9.9a)失败")?;
        assert!(resp.sessions.is_empty(), "缺省 sessions 应空响应: {resp:?}");
        let resp =
            sync_sessions(&test_rb, Some(sync_user.to_string()), sync_req(Some(vec![]), None))
                .await
                .context("同步(9.9b)失败")?;
        assert!(resp.sessions.is_empty(), "空 sessions 应空响应: {resp:?}");

        // 9.10 群聊会话: session_uuid=group_uuid, recv_user=group_uuid, text_type=msg_type
        let group_uuid = Uuid::now_v7();
        let group_uuid_rbdc: RbatisUuid = group_uuid.to_string().parse()?;
        GroupInfo::insert(
            &test_rb,
            &GroupInfo {
                id: None,
                group_uuid: Some(group_uuid_rbdc.clone()),
                group_name: Some("同步测试群".to_string()),
                avatar: None,
                owner_uuid: Some(sync_user_rbdc.clone()),
                description: None,
                max_members: Some(200),
                created_at: Some(0),
                updated_at: Some(0),
                status: Some(1),
            },
        )
        .await
        .context("插入同步测试群失败")?;
        let mut ids_g = Vec::new();
        for i in 0..3 {
            ids_g.push(
                insert_group_msg(
                    &test_rb,
                    &group_uuid_rbdc,
                    &format!("sync-g-{i}"),
                    sync_now + i,
                    &sync_user_rbdc,
                )
                .await?,
            );
        }
        upsert_user_session(
            &test_rb,
            &sync_user_rbdc,
            &group_uuid_rbdc,
            SESSION_TYPE_GROUP,
            None,
            0,
        )
        .await?;
        let resp = sync_sessions(
            &test_rb,
            Some(sync_user.to_string()),
            sync_req(Some(vec![sync_session_req(&group_uuid_rbdc)]), Some(2)),
        )
        .await
        .context("同步(9.10)失败")?;
        let s = &resp.sessions[0];
        assert_eq!(s.session_type, SESSION_TYPE_GROUP);
        assert_eq!(s.messages.len(), 2);
        assert!(s.has_more, "还有 1 条 → has_more=true");
        assert_eq!(s.messages[0].recv_user, group_uuid.to_string(), "群聊 recv_user=group_uuid");
        assert_eq!(s.messages[0].id, ids_g[0], "正向首拉取最旧 2 条");
        assert_eq!(s.next_cursor, ids_g[1], "群聊 next_cursor=本批最大 id");

        // 9.11 HTTP 冒烟: 新契约(对象化 sessions)路由注册与响应结构
        let sync_body = serde_json::from_str::<JsonValue>(r#"{"sessions":[],"limit":10}"#)
            .context("构造冒烟请求失败")?;
        let (status, json) =
            post_json(&app, "/session/sync", Some(&sync_body), Some(&access_token)).await;
        assert_eq!(status, StatusCode::OK, "/session/sync 应成功: {json}");
        assert_eq!(json["code"], 200, "/session/sync 响应 code 应为 200: {json}");
        assert!(json["data"]["server_time"].as_i64().is_some(), "应返回 server_time: {json}");
        assert!(json["data"]["sessions"].as_array().is_some(), "应返回 sessions 数组: {json}");
        info!("任务12 离线同步 /session/sync 全部通过");

        // ===== 10. 任务06: 会话列表与控制信息 =====
        let base = sync_now;

        // 10.1 排序: 置顶最前, 其余按 last_message_at 降序
        let (u1, u1r) = seed_friend_user(&test_rb, "l1").await?;
        let p1a = Uuid::now_v7();
        let p1b = Uuid::now_v7();
        let p1c = Uuid::now_v7();
        let p1ar: RbatisUuid = p1a.to_string().parse()?;
        let p1br: RbatisUuid = p1b.to_string().parse()?;
        let p1cr: RbatisUuid = p1c.to_string().parse()?;
        let s1a: RbatisUuid = single_session_uuid(&u1, &p1a).to_string().parse()?;
        let s1b: RbatisUuid = single_session_uuid(&u1, &p1b).to_string().parse()?;
        let s1c: RbatisUuid = single_session_uuid(&u1, &p1c).to_string().parse()?;
        // s1a 置顶(时间最早); s1b 最新; s1c 居中
        upsert_session_row(&test_rb, &s1a, SESSION_TYPE_SINGLE, 1, base - 2000, "a").await?;
        upsert_user_session_ex(&test_rb, &u1r, &s1a, SESSION_TYPE_SINGLE, Some(&p1ar), 0, 1, 1, 0)
            .await?;
        upsert_session_row(&test_rb, &s1b, SESSION_TYPE_SINGLE, 1, base - 100, "b").await?;
        upsert_user_session_ex(&test_rb, &u1r, &s1b, SESSION_TYPE_SINGLE, Some(&p1br), 0, 1, 0, 0)
            .await?;
        upsert_session_row(&test_rb, &s1c, SESSION_TYPE_SINGLE, 1, base - 1000, "c").await?;
        upsert_user_session_ex(&test_rb, &u1r, &s1c, SESSION_TYPE_SINGLE, Some(&p1cr), 0, 1, 0, 0)
            .await?;
        let resp = list_sessions(&test_rb, Some(u1.to_string()), list_req(None, Some(10))).await?;
        let order1: Vec<String> = resp.sessions.iter().map(|s| s.session_uuid.clone()).collect();
        assert_eq!(
            order1,
            vec![s1a.to_string(), s1b.to_string(), s1c.to_string()],
            "置顶最前, 其余按时间降序"
        );
        assert!(!resp.has_more);
        assert!(resp.next_cursor.is_none());

        // 10.2 keyset 分页无漏无重(12 会话 / size=5 → 3 页)
        let (u2, u2r) = seed_friend_user(&test_rb, "l2").await?;
        let mut expect2 = Vec::new();
        for i in 0..12i64 {
            let peer = Uuid::now_v7();
            let pr: RbatisUuid = peer.to_string().parse()?;
            let s: RbatisUuid = single_session_uuid(&u2, &peer).to_string().parse()?;
            upsert_session_row(&test_rb, &s, SESSION_TYPE_SINGLE, i + 1, base - i * 1000, "k")
                .await?;
            upsert_user_session_ex(&test_rb, &u2r, &s, SESSION_TYPE_SINGLE, Some(&pr), 0, 0, 0, 0)
                .await?;
            expect2.push(s.to_string());
        }
        let mut got2 = Vec::new();
        let mut cursor2: Option<SessionListCursor> = None;
        loop {
            let resp =
                list_sessions(&test_rb, Some(u2.to_string()), list_req(cursor2, Some(5))).await?;
            got2.extend(resp.sessions.iter().map(|s| s.session_uuid.clone()));
            if !resp.has_more {
                break;
            }
            cursor2 = resp.next_cursor;
        }
        assert_eq!(got2.len(), 12, "翻页合并应 12 行");
        let mut uniq2 = got2.clone();
        uniq2.sort();
        uniq2.dedup();
        assert_eq!(uniq2.len(), 12, "翻页不应有重复");
        let mut exp2 = expect2.clone();
        exp2.sort();
        assert_eq!(uniq2, exp2, "翻页集合应与预期一致");

        // 10.3 未读数排除自己发的(单聊 recv=me / 群 send<>me)
        let (u3, u3r) = seed_friend_user(&test_rb, "l3").await?;
        let p3 = Uuid::now_v7();
        let p3r: RbatisUuid = p3.to_string().parse()?;
        let s3: RbatisUuid = single_session_uuid(&u3, &p3).to_string().parse()?;
        for i in 0..3i64 {
            insert_single_msg(&test_rb, &s3, &format!("u3-me-{i}"), base + i, &u3r, &p3r).await?;
        }
        for i in 0..2i64 {
            insert_single_msg(&test_rb, &s3, &format!("u3-peer-{i}"), base + 10 + i, &p3r, &u3r)
                .await?;
        }
        let max3 = ChatMessageRecord::max_id_by_session(&test_rb, &s3).await?;
        upsert_session_row(&test_rb, &s3, SESSION_TYPE_SINGLE, max3, base, "u3").await?;
        upsert_user_session_ex(&test_rb, &u3r, &s3, SESSION_TYPE_SINGLE, Some(&p3r), 0, max3, 0, 0)
            .await?;

        let g3 = Uuid::now_v7();
        let g3r: RbatisUuid = g3.to_string().parse()?;
        GroupInfo::insert(
            &test_rb,
            &GroupInfo {
                id: None,
                group_uuid: Some(g3r.clone()),
                group_name: Some("列表测试群".to_string()),
                avatar: None,
                owner_uuid: Some(u3r.clone()),
                description: None,
                max_members: Some(200),
                created_at: Some(0),
                updated_at: Some(0),
                status: Some(1),
            },
        )
        .await
        .context("插入列表测试群失败")?;
        insert_group_msg(&test_rb, &g3r, "g3-me-0", base, &u3r).await?;
        let other3 = Uuid::now_v7();
        let other3r: RbatisUuid = other3.to_string().parse()?;
        for i in 0..4i64 {
            insert_group_msg(&test_rb, &g3r, &format!("g3-o-{i}"), base + 10 + i, &other3r).await?;
        }
        let gmax3 = GroupMessageRecord::max_id_by_group(&test_rb, &g3r).await?;
        upsert_session_row(&test_rb, &g3r, SESSION_TYPE_GROUP, gmax3, base, "g3").await?;
        upsert_user_session_ex(&test_rb, &u3r, &g3r, SESSION_TYPE_GROUP, None, 0, gmax3, 0, 0)
            .await?;

        let resp = list_sessions(&test_rb, Some(u3.to_string()), list_req(None, Some(10))).await?;
        let single3 = resp
            .sessions
            .iter()
            .find(|s| s.session_uuid == s3.to_string())
            .context("应有单聊会话")?;
        assert_eq!(single3.unread, 2, "单聊未读=对方发的 2 条");
        let group3 = resp
            .sessions
            .iter()
            .find(|s| s.session_uuid == g3.to_string())
            .context("应有群会话")?;
        assert_eq!(group3.unread, 4, "群未读=他人发的 4 条");

        // 10.4 软删后新消息复活, 未读=新到条数
        let (u4, u4r) = seed_friend_user(&test_rb, "l4").await?;
        let p4 = Uuid::now_v7();
        let p4r: RbatisUuid = p4.to_string().parse()?;
        let s4: RbatisUuid = single_session_uuid(&u4, &p4).to_string().parse()?;
        insert_single_msg(&test_rb, &s4, "r4-0", base, &p4r, &u4r).await?;
        insert_single_msg(&test_rb, &s4, "r4-1", base + 1, &p4r, &u4r).await?;
        let max4 = ChatMessageRecord::max_id_by_session(&test_rb, &s4).await?;
        upsert_session_row(&test_rb, &s4, SESSION_TYPE_SINGLE, max4, base, "r4").await?;
        upsert_user_session_ex(&test_rb, &u4r, &s4, SESSION_TYPE_SINGLE, Some(&p4r), 0, max4, 0, 0)
            .await?;
        delete_session(&test_rb, Some(u4.to_string()), control_req(&s4, None))
            .await
            .context("软删失败")?;
        let us4 = UserSession::select_by_map(
            &test_rb,
            rbs::value! {"user_uuid": &u4r, "session_uuid": &s4},
        )
        .await?;
        let deleted_at4 = us4[0].deleted_at.context("软删后 deleted_at 应非空")?;
        assert_eq!(us4[0].last_read_id, Some(max4), "软删应推到底");
        let resp = list_sessions(&test_rb, Some(u4.to_string()), list_req(None, None)).await?;
        assert!(resp.sessions.iter().all(|s| s.session_uuid != s4.to_string()), "软删后不应出现");
        // 新消息 + 聚合 → 复活
        insert_single_msg(&test_rb, &s4, "r4-2", deleted_at4 + 1000, &p4r, &u4r).await?;
        aggregate_user_sessions(&test_rb, &u4r).await.context("复活聚合失败")?;
        let resp = list_sessions(&test_rb, Some(u4.to_string()), list_req(None, None)).await?;
        let row4 =
            resp.sessions.iter().find(|s| s.session_uuid == s4.to_string()).context("应复活")?;
        assert_eq!(row4.unread, 1, "复活后未读=新到 1 条");

        // 10.5 软删后无新消息保持隐藏
        let (u5, u5r) = seed_friend_user(&test_rb, "l5").await?;
        let p5 = Uuid::now_v7();
        let p5r: RbatisUuid = p5.to_string().parse()?;
        let s5: RbatisUuid = single_session_uuid(&u5, &p5).to_string().parse()?;
        insert_single_msg(&test_rb, &s5, "h5-0", base, &p5r, &u5r).await?;
        let max5 = ChatMessageRecord::max_id_by_session(&test_rb, &s5).await?;
        upsert_session_row(&test_rb, &s5, SESSION_TYPE_SINGLE, max5, base, "h5").await?;
        upsert_user_session_ex(&test_rb, &u5r, &s5, SESSION_TYPE_SINGLE, Some(&p5r), 0, max5, 0, 0)
            .await?;
        delete_session(&test_rb, Some(u5.to_string()), control_req(&s5, None)).await?;
        let resp = list_sessions(&test_rb, Some(u5.to_string()), list_req(None, None)).await?;
        assert!(resp.sessions.is_empty(), "软删且无新消息应始终隐藏");

        // 10.6 preview 生成: 300 汉字 → 256 字符, UTF-8 安全
        let (u6, u6r) = seed_friend_user(&test_rb, "l6").await?;
        let p6 = Uuid::now_v7();
        let p6r: RbatisUuid = p6.to_string().parse()?;
        let s6: RbatisUuid = single_session_uuid(&u6, &p6).to_string().parse()?;
        let long6 = "汉".repeat(300);
        ChatMessageRecord::insert(
            &test_rb,
            &ChatMessageRecord {
                id: None,
                session_uuid: s6.clone(),
                nano_id: Some("preview6".to_string()),
                timestamp: Some(base),
                raw: long6.as_bytes().to_vec().into(),
                text_type: Some(0),
                send_user: p6r.clone(),
                recv_user: u6r.clone(),
            },
        )
        .await
        .context("插入长消息失败")?;
        aggregate_user_sessions(&test_rb, &u6r).await.context("preview 聚合失败")?;
        let resp = list_sessions(&test_rb, Some(u6.to_string()), list_req(None, None)).await?;
        let row6 =
            resp.sessions.iter().find(|s| s.session_uuid == s6.to_string()).context("应有会话")?;
        assert_eq!(row6.last_preview.chars().count(), 256, "preview 应截断到 256 字符");
        assert!(row6.last_preview.chars().all(|c| c == '汉'), "preview 应无乱码(UTF-8 安全)");

        // 10.7 pin/mute/delete 落库
        let (u7, u7r) = seed_friend_user(&test_rb, "l7").await?;
        let p7 = Uuid::now_v7();
        let p7r: RbatisUuid = p7.to_string().parse()?;
        let s7: RbatisUuid = single_session_uuid(&u7, &p7).to_string().parse()?;
        insert_single_msg(&test_rb, &s7, "pmd7", base, &p7r, &u7r).await?;
        let max7 = ChatMessageRecord::max_id_by_session(&test_rb, &s7).await?;
        upsert_session_row(&test_rb, &s7, SESSION_TYPE_SINGLE, max7, base, "pmd7").await?;
        upsert_user_session_ex(&test_rb, &u7r, &s7, SESSION_TYPE_SINGLE, Some(&p7r), 0, max7, 0, 0)
            .await?;
        pin_session(&test_rb, Some(u7.to_string()), control_req(&s7, Some(1)))
            .await
            .context("置顶失败")?;
        mute_session(&test_rb, Some(u7.to_string()), control_req(&s7, Some(1)))
            .await
            .context("免打扰失败")?;
        let us7 = UserSession::select_by_map(
            &test_rb,
            rbs::value! {"user_uuid": &u7r, "session_uuid": &s7},
        )
        .await?;
        assert_eq!(us7[0].pinned, Some(1), "置顶应落库");
        assert_eq!(us7[0].muted, Some(1), "免打扰应落库");
        delete_session(&test_rb, Some(u7.to_string()), control_req(&s7, None))
            .await
            .context("软删失败")?;
        let us7 = UserSession::select_by_map(
            &test_rb,
            rbs::value! {"user_uuid": &u7r, "session_uuid": &s7},
        )
        .await?;
        assert!(us7[0].deleted_at.is_some(), "软删应落 deleted_at");
        assert_eq!(us7[0].last_read_id, Some(max7), "软删应把 last_read 推到 last_message_id");

        // 10.8 无会话用户返回空
        let (u8, _u8r) = seed_friend_user(&test_rb, "l8").await?;
        let resp = list_sessions(&test_rb, Some(u8.to_string()), list_req(None, None)).await?;
        assert!(resp.sessions.is_empty());
        assert!(!resp.has_more);
        assert!(resp.next_cursor.is_none());

        // 10.9 HTTP 冒烟: 路由与响应结构
        let list_body = json_obj(&[]);
        let (status, json) =
            post_json(&app, "/session/list", Some(&list_body), Some(&access_token)).await;
        assert_eq!(status, StatusCode::OK, "/session/list 应成功: {json}");
        assert_eq!(json["code"], 200, "/session/list code 应为 200: {json}");
        assert!(json["data"]["sessions"].as_array().is_some(), "应返回 sessions 数组: {json}");
        assert!(json["data"]["has_more"].as_bool().is_some(), "应返回 has_more: {json}");
        info!("任务06 会话列表与控制信息 全部通过");

        Ok::<(), anyhow::Error>(())
    })
    .catch_unwind()
    .await;

    // ===== 清理（无论测试成功还是失败都执行）=====
    if let Err(e) = flush_test_redis(&redis_pool).await {
        info!("清空测试 Redis 失败（不影响测试结果）: {}", e);
    }
    drop_test_database(&admin, TEST_DATABASE_NAME).await?;

    match outcome {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(e),
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

/// 任务04b: 直接写入一个可被好友流程引用的最小用户(basic_user 即满足 is_exist_user_by_uuid)。
async fn seed_friend_user(rb: &RBatis, tag: &str) -> Result<(Uuid, RbatisUuid)> {
    let uuid = Uuid::now_v7();
    let rbdc: RbatisUuid = uuid.to_string().parse().context("解析用户 UUID 失败")?;
    BasicUser::insert(
        rb,
        &BasicUser {
            uuid: Some(rbdc.clone()),
            username: Some(format!("friend_{tag}")),
            account: Some(format!("friend_{tag}")),
            icon: None,
            info: Some(String::new()),
            password: Some("FriendSeedPass123456".to_string()),
            registration_status: Some(1),
            user_type: Some(0),
        },
    )
    .await
    .context("写入好友测试用户失败")?;
    Ok((uuid, rbdc))
}

/// 好友流程 DTO 构造(申请与接受共用)。
fn friend_dto(
    request_user: &Uuid,
    accept_user: &Uuid,
    request_message: Option<&str>,
    accept_message: Option<&str>,
    accept_status: Option<u8>,
) -> FriendRequestInfoDTO {
    FriendRequestInfoDTO {
        request_message: request_message.map(str::to_string),
        accept_message: accept_message.map(str::to_string),
        request_user: Some(request_user.to_string()),
        accept_user: Some(accept_user.to_string()),
        add_type: Some("search".to_string()),
        version: None,
        accept_status,
    }
}

/// 某会话的全部消息, 按 id 升序(会话内对话顺序)。
async fn select_session_messages(
    rb: &RBatis,
    session_uuid: &RbatisUuid,
) -> Result<Vec<ChatMessageRecord>> {
    let mut msgs = ChatMessageRecord::select_by_map(rb, rbs::value! {"session_uuid": session_uuid})
        .await
        .context("查询会话消息失败")?;
    msgs.sort_by_key(|m| m.id.unwrap_or(0));
    Ok(msgs)
}

/// 消息 raw 内容(UTF-8)。
fn msg_text(msg: &ChatMessageRecord) -> Result<String> {
    Ok(String::from_utf8(msg.raw.clone().into_inner())?)
}

/// 任务05: 插入一条单聊同步测试消息, 返回会话当前最大 id。
async fn insert_single_msg(
    rb: &RBatis,
    session: &RbatisUuid,
    nano: &str,
    ts: i64,
    send: &RbatisUuid,
    recv: &RbatisUuid,
) -> Result<i64> {
    ChatMessageRecord::insert(
        rb,
        &ChatMessageRecord {
            id: None,
            session_uuid: session.clone(),
            nano_id: Some(nano.to_string()),
            timestamp: Some(ts),
            raw: b"m".to_vec().into(),
            text_type: Some(0),
            send_user: send.clone(),
            recv_user: recv.clone(),
        },
    )
    .await
    .context("插入同步测试消息失败")?;
    ChatMessageRecord::max_id_by_session(rb, session).await.context("取会话最大 id 失败")
}

/// 任务05: 建/取用户会话行(带指定同步游标)。
async fn upsert_user_session(
    rb: &RBatis,
    me: &RbatisUuid,
    session: &RbatisUuid,
    session_type: i16,
    peer: Option<&RbatisUuid>,
    synced_id: i64,
) -> Result<()> {
    upsert_user_session_ex(rb, me, session, session_type, peer, 0, synced_id, 0, 0).await
}

/// 任务06: 建/取用户会话行(游标 + 置顶 + 免打扰全量可控)。
#[allow(clippy::too_many_arguments)]
async fn upsert_user_session_ex(
    rb: &RBatis,
    me: &RbatisUuid,
    session: &RbatisUuid,
    session_type: i16,
    peer: Option<&RbatisUuid>,
    last_read_id: i64,
    synced_id: i64,
    pinned: i16,
    muted: i16,
) -> Result<()> {
    UserSession::upsert(
        rb,
        &UserSession {
            id: None,
            user_uuid: me.clone(),
            session_uuid: session.clone(),
            session_type: Some(session_type),
            peer_uuid: peer.cloned(),
            last_read_id: Some(last_read_id),
            synced_id: Some(synced_id),
            pinned: Some(pinned),
            muted: Some(muted),
            deleted_at: None,
            created_at: None,
            updated_at: None,
        },
    )
    .await
    .context("写入测试 user_session 失败")?;
    // UserSession::upsert 是懒创建(仅 8 列, 不写 pinned/muted), 这里按需补控制字段
    if pinned != 0 {
        UserSession::update_pinned(rb, me, session, pinned).await.context("设置测试置顶失败")?;
    }
    if muted != 0 {
        UserSession::update_muted(rb, me, session, muted).await.context("设置测试免打扰失败")?;
    }
    Ok(())
}

/// 任务06: 建 session 行并设置最后一条消息(列表排序/摘要依赖)。
async fn upsert_session_row(
    rb: &RBatis,
    session: &RbatisUuid,
    session_type: i16,
    last_id: i64,
    last_at: i64,
    preview: &str,
) -> Result<()> {
    Session::upsert(
        rb,
        &Session {
            session_uuid: session.clone(),
            session_type: Some(session_type),
            last_message_id: None,
            last_message_at: None,
            last_preview: None,
            created_at: None,
            updated_at: None,
        },
    )
    .await
    .context("写入测试 session 失败")?;
    Session::update_last_message(rb, session, last_id, last_at, Some(preview))
        .await
        .context("更新测试 session 摘要失败")?;
    Ok(())
}

/// 任务06: 插入一条群聊测试消息, 返回群当前最大 id。
async fn insert_group_msg(
    rb: &RBatis,
    group_uuid: &RbatisUuid,
    nano: &str,
    ts: i64,
    send: &RbatisUuid,
) -> Result<i64> {
    GroupMessageRecord::insert(
        rb,
        &GroupMessageRecord {
            id: None,
            nano_id: Some(nano.to_string()),
            group_uuid: Some(group_uuid.clone()),
            send_user: Some(send.clone()),
            timestamp: Some(ts),
            raw: b"g".to_vec().into(),
            msg_type: Some(MSG_TYPE_TEXT),
            recalled: Some(false),
        },
    )
    .await
    .context("插入群测试消息失败")?;
    GroupMessageRecord::max_id_by_group(rb, group_uuid).await.context("取群最大 id 失败")
}

/// 任务06: 会话列表请求构造。
fn list_req(cursor: Option<SessionListCursor>, size: Option<u32>) -> SessionListDTO {
    SessionListDTO { cursor, size }
}

/// 任务06: 控制信息请求构造。
fn control_req(session: &RbatisUuid, value: Option<i16>) -> SessionControlDTO {
    SessionControlDTO { session_uuid: session.to_string(), value }
}

/// 任务12: 同步请求构造(会话对象化)。
fn sync_req(sessions: Option<Vec<SyncSessionReq>>, limit: Option<u32>) -> SyncRequestDTO {
    SyncRequestDTO { sessions, limit }
}

/// 任务12: 首拉会话项(无 after_id, 服务端视作 0 = 窗口内最早起)。
fn sync_session_req(session: &RbatisUuid) -> SyncSessionReq {
    SyncSessionReq { session_uuid: session.to_string(), after_id: None }
}

/// 任务12: 正向续拉会话项(带 after_id)。
fn sync_session_req_after(session: &RbatisUuid, after_id: i64) -> SyncSessionReq {
    SyncSessionReq { session_uuid: session.to_string(), after_id: Some(after_id) }
}

/// 构造字符串键值对的 JSON 对象（不使用 `serde_json::json!` 宏，因其内部调用 `unwrap` 违反仓库规范）
fn json_obj(pairs: &[(&str, &str)]) -> JsonValue {
    let mut map = serde_json::Map::new();
    for (k, v) in pairs {
        map.insert(k.to_string(), JsonValue::String(v.to_string()));
    }
    JsonValue::Object(map)
}

/// 发送 POST 请求并返回 (HTTP 状态码, 解析后的 JSON 响应)
async fn post_json<S, B>(
    app: &S,
    uri: &str,
    body: Option<&JsonValue>,
    token: Option<&str>,
) -> (StatusCode, JsonValue)
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    B: actix_web::body::MessageBody,
{
    let mut req = test::TestRequest::post().uri(uri);
    if let Some(b) = body {
        req = req.set_json(b);
    }
    if let Some(t) = token {
        req = req.insert_header(("Authorization", t));
    }
    let resp = test::call_service(app, req.to_request()).await;
    let status = resp.status();
    let json: JsonValue =
        serde_json::from_slice(&test::read_body(resp).await).unwrap_or(JsonValue::Null);
    (status, json)
}

/// 发送 GET 请求并返回 (HTTP 状态码, 解析后的 JSON 响应)
async fn get_json<S, B>(app: &S, uri: &str, token: Option<&str>) -> (StatusCode, JsonValue)
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    B: actix_web::body::MessageBody,
{
    let mut req = test::TestRequest::get().uri(uri);
    if let Some(t) = token {
        req = req.insert_header(("Authorization", t));
    }
    let resp = test::call_service(app, req.to_request()).await;
    let status = resp.status();
    let json: JsonValue =
        serde_json::from_slice(&test::read_body(resp).await).unwrap_or(JsonValue::Null);
    (status, json)
}

/// 初始化 tracing，日志输出到 stdout（默认 info 级别，可用 RUST_LOG 覆盖）
fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = fmt().with_env_filter(filter).with_writer(std::io::stdout).try_init();
}

/// 脱敏打印连接串，避免把密码输出到日志
fn mask_url(url: &str) -> String {
    match url.split_once('@') {
        Some((_, tail)) => format!("***@{}", tail),
        None => "***".to_string(),
    }
}

/// 将管理员连接 URL 的库名替换为测试库名，其余连接信息保持不变
fn test_database_url(admin_url: &str, db_name: &str) -> Result<String> {
    validate_db_name(db_name)?;
    let at = admin_url.rfind('@').ok_or_else(|| anyhow!("无效的 DATABASE_URL: {}", admin_url))?;
    let (head, tail) = admin_url.split_at(at + 1);
    let slash =
        tail.find('/').ok_or_else(|| anyhow!("DATABASE_URL 缺少数据库名: {}", admin_url))?;
    Ok(format!("{}{}/{}", head, &tail[..slash], db_name))
}

/// 删除并重建测试库（若不存在则直接创建）。
///
/// 每次运行都必须从空库开始：分区表无法由普通表原地转换
/// （`CREATE TABLE IF NOT EXISTS ... PARTITION BY HASH` 遇到已存在的普通表会静默跳过，
/// 随后 `CREATE TABLE ... PARTITION OF` 因父表不是分区表而失败）。复用旧库会让 DDL 必失败。
async fn recreate_database(admin: &RBatis, db_name: &str) -> Result<()> {
    validate_db_name(db_name)?;
    drop_test_database(admin, db_name).await?;
    let create_sql = format!("CREATE DATABASE \"{}\"", db_name);
    admin
        .exec(&create_sql, vec![])
        .await
        .map_err(|e| anyhow!("创建测试库 {} 失败: {}", db_name, e))?;
    info!("已创建测试库 {}", db_name);
    Ok(())
}

/// 删除测试库：先终止测试库上的所有连接，再执行 `DROP DATABASE`
async fn drop_test_database(admin: &RBatis, db_name: &str) -> Result<()> {
    validate_db_name(db_name)?;
    let terminate_sql = format!(
        "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '{}' AND pid <> pg_backend_pid()",
        db_name
    );
    admin
        .query(&terminate_sql, vec![])
        .await
        .map_err(|e| anyhow!("终止测试库 {} 的连接失败: {}", db_name, e))?;
    let drop_sql = format!("DROP DATABASE IF EXISTS \"{}\"", db_name);
    admin
        .exec(&drop_sql, vec![])
        .await
        .map_err(|e| anyhow!("删除测试库 {} 失败: {}", db_name, e))?;
    info!("测试库 {} 已删除", db_name);
    Ok(())
}

/// 查询表是否存在
async fn table_exists(rb: &RBatis, table: &str) -> Result<bool> {
    let sql = format!(
        "SELECT 1 FROM information_schema.tables WHERE table_schema = 'public' AND table_name = '{}'",
        table
    );
    let result: rbs::Value =
        rb.query(&sql, vec![]).await.map_err(|e| anyhow!("查询表信息失败: {}", e))?;
    Ok(result.as_array().map_or(0, |rows| rows.len()) > 0)
}

/// 校验库名只能包含字母、数字、下划线
fn validate_db_name(db_name: &str) -> Result<()> {
    if db_name.is_empty() || !db_name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(anyhow!("非法测试库名: {}", db_name));
    }
    Ok(())
}

/// 构建 rbatis 连接池
async fn build_db_pool(url: &str) -> Result<RBatis> {
    let rb = RBatis::new();
    let mut opts = PgConnectOptions::new();
    opts.set_uri(url).map_err(|e| anyhow!("设置数据库 URI 失败: {}", e))?;
    let conn_manager = ConnectionManager::new_options(PgDriver {}, opts);
    let pool = FastPool::new(conn_manager).map_err(|e| anyhow!("创建连接池失败: {}", e))?;
    pool.set_timeout(Some(Duration::from_secs(2))).await;
    rb.pool
        .set(Box::new(pool))
        .map_err(|_e| rbatis::Error::from("pool set fail!"))
        .map_err(|e| anyhow!("设置连接池失败: {}", e))?;
    Ok(rb)
}

/// 构建 Redis 连接池（独立于 common 的全局单例，便于测试隔离）
fn build_redis_pool(url: &str) -> Result<Pool> {
    let config = RedisConfig::from_url(url);
    let pool = config
        .create_pool(Some(Runtime::Tokio1))
        .map_err(|e| anyhow!("创建 Redis 连接池失败: {}", e))?;
    Ok(pool)
}

/// 验证测试 Redis 连通性
async fn verify_redis_pool(pool: &Pool) -> Result<()> {
    let mut conn = pool.get().await.map_err(|e| anyhow!("获取 Redis 连接失败: {}", e))?;
    let pong: String =
        cmd("PING").query_async(&mut conn).await.map_err(|e| anyhow!("Redis PING 失败: {}", e))?;
    info!("测试 Redis 连接成功 (PING: {})", pong);
    Ok(())
}

/// 清空测试 Redis 的所有 key
async fn flush_test_redis(pool: &Pool) -> Result<()> {
    let mut conn = pool.get().await.map_err(|e| anyhow!("获取 Redis 连接失败: {}", e))?;
    cmd("FLUSHALL")
        .query_async::<()>(&mut conn)
        .await
        .map_err(|e| anyhow!("清空测试 Redis key 失败: {}", e))?;
    info!("测试 Redis 所有 key 已清空 (FLUSHALL)");
    Ok(())
}
