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
use common::models::session_entity::session::{SESSION_TYPE_GROUP, SESSION_TYPE_SINGLE};
use common::models::session_entity::user_session::UserSession;
use common::models::user_entity::basic_user::BasicUser;
use common::models::user_entity::email_sso::EmailSso;
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

        // ===== 7. 任务04: 已读桥接 / 缺陷A / session 接口 =====
        // 7.1 桥接单聊: 旧 DTO 上报 → 直写 user_session 游标(读到底, synced 同步推进)
        let peer_a = Uuid::new_v4();
        let peer_a_rbdc: RbatisUuid = peer_a.to_string().parse()?;
        let session_a: RbatisUuid = single_session_uuid(&seed_uuid, &peer_a).to_string().parse()?;
        for ts in [1_000i64, 2_000] {
            ChatMessageRecord::insert(
                &test_rb,
                &ChatMessageRecord {
                    id: None,
                    session_uuid: session_a.clone(),
                    nano_id: Some(format!("bridge-a-{ts}")),
                    timestamp: Some(ts),
                    raw: b"hi".to_vec().into(),
                    text_type: Some(1),
                    send_user: peer_a_rbdc.clone(),
                    recv_user: seed_uuid_rbdc.clone(),
                },
            )
            .await
            .context("插入桥接单聊消息失败")?;
        }
        let expected_a = ChatMessageRecord::max_id_by_session(&test_rb, &session_a).await?;

        // 构造旧契约已读上报体(单聊: send/recv + chat_type=1; 群聊 chat_type=2)
        let bridge_read_body = |peer: &Uuid, nano: &str, ts: i64, chat_type: i64| {
            let mut m = serde_json::Map::new();
            m.insert("nano_id".into(), JsonValue::String(nano.to_string()));
            m.insert("timestamp".into(), JsonValue::from(ts));
            m.insert("send_user".into(), JsonValue::String(peer.to_string()));
            m.insert("recv_user".into(), JsonValue::String(seed_uuid.to_string()));
            m.insert("chat_type".into(), JsonValue::from(chat_type));
            JsonValue::Array(vec![JsonValue::Object(m)])
        };

        let (status, json) = post_json(
            &app,
            "/msg/add_read_chat_record",
            Some(&bridge_read_body(&peer_a, "bridge-a-2000", 2_000, 1)),
            Some(&access_token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "桥接单聊已读上报应成功: {json}");

        let us_a = UserSession::select_by_map(
            &test_rb,
            rbs::value! {"user_uuid": &seed_uuid_rbdc, "session_uuid": &session_a},
        )
        .await
        .context("查询桥接 user_session 失败")?;
        assert_eq!(us_a.len(), 1, "桥接应建立 user_session 行");
        assert_eq!(us_a[0].last_read_id, Some(expected_a), "桥接应把已读游标推到 max");
        assert_eq!(us_a[0].synced_id, Some(expected_a), "桥接应同步推进 synced_id");
        assert_eq!(us_a[0].peer_uuid, Some(peer_a_rbdc.clone()), "桥接应记录对方 uuid");

        // 再发一条新消息 → 二次上报 → 游标前进
        ChatMessageRecord::insert(
            &test_rb,
            &ChatMessageRecord {
                id: None,
                session_uuid: session_a.clone(),
                nano_id: Some("bridge-a-3000".to_string()),
                timestamp: Some(3_000),
                raw: b"hi2".to_vec().into(),
                text_type: Some(1),
                send_user: peer_a_rbdc.clone(),
                recv_user: seed_uuid_rbdc.clone(),
            },
        )
        .await
        .context("插入第二条桥接消息失败")?;
        let expected_a2 = ChatMessageRecord::max_id_by_session(&test_rb, &session_a).await?;
        assert!(expected_a2 > expected_a, "新消息 id 应更大");
        let (status, _json) = post_json(
            &app,
            "/msg/add_read_chat_record",
            Some(&bridge_read_body(&peer_a, "bridge-a-3000", 3_000, 1)),
            Some(&access_token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "二次桥接上报应成功");
        let us_a2 = UserSession::select_by_map(
            &test_rb,
            rbs::value! {"user_uuid": &seed_uuid_rbdc, "session_uuid": &session_a},
        )
        .await
        .context("二次查询桥接 user_session 失败")?;
        assert_eq!(us_a2[0].last_read_id, Some(expected_a2), "二次上报游标应前进");

        // 7.2 桥接群聊: 旧 DTO(chat_type=2) → 群会话游标推进
        let group_uuid = Uuid::new_v4();
        let group_uuid_rbdc: RbatisUuid = group_uuid.to_string().parse()?;
        let owner = Uuid::new_v4();
        let owner_rbdc: RbatisUuid = owner.to_string().parse()?;
        GroupInfo::insert(
            &test_rb,
            &GroupInfo {
                id: None,
                group_uuid: Some(group_uuid_rbdc.clone()),
                group_name: Some("桥接测试群".to_string()),
                avatar: None,
                owner_uuid: Some(owner_rbdc.clone()),
                description: None,
                max_members: Some(200),
                created_at: Some(0),
                updated_at: Some(0),
                status: Some(1),
            },
        )
        .await
        .context("插入桥接群信息失败")?;
        for ts in [1_000i64, 2_000] {
            GroupMessageRecord::insert(
                &test_rb,
                &GroupMessageRecord {
                    id: None,
                    nano_id: Some(format!("bridge-g-{ts}")),
                    group_uuid: Some(group_uuid_rbdc.clone()),
                    send_user: Some(owner_rbdc.clone()),
                    timestamp: Some(ts),
                    raw: b"gm".to_vec().into(),
                    msg_type: Some(MSG_TYPE_TEXT),
                    recalled: Some(false),
                },
            )
            .await
            .context("插入桥接群消息失败")?;
        }
        let expected_g = GroupMessageRecord::max_id_by_group(&test_rb, &group_uuid_rbdc).await?;
        let (status, json) = post_json(
            &app,
            "/msg/add_read_chat_record",
            Some(&bridge_read_body(&owner, "bridge-g-2000", 2_000, 2)),
            Some(&access_token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "桥接群聊已读上报应成功: {json}");
        let us_g = UserSession::select_by_map(
            &test_rb,
            rbs::value! {"user_uuid": &seed_uuid_rbdc, "session_uuid": &group_uuid_rbdc},
        )
        .await
        .context("查询桥接群 user_session 失败")?;
        assert_eq!(us_g.len(), 1, "群桥接应建立 user_session 行");
        assert_eq!(us_g[0].session_type, Some(SESSION_TYPE_GROUP));
        assert_eq!(us_g[0].last_read_id, Some(expected_g), "群桥接游标应到 max");
        assert_eq!(us_g[0].synced_id, Some(expected_g));

        // 7.3 缺陷A回归: 读取 C 会话不应丢掉 B 会话的未读(旧全局时间戳会丢)
        let peer_b = Uuid::new_v4();
        let peer_c = Uuid::new_v4();
        let peer_b_rbdc: RbatisUuid = peer_b.to_string().parse()?;
        let peer_c_rbdc: RbatisUuid = peer_c.to_string().parse()?;
        let session_ab: RbatisUuid =
            single_session_uuid(&seed_uuid, &peer_b).to_string().parse()?;
        let session_ac: RbatisUuid =
            single_session_uuid(&seed_uuid, &peer_c).to_string().parse()?;
        ChatMessageRecord::insert(
            &test_rb,
            &ChatMessageRecord {
                id: None,
                session_uuid: session_ab.clone(),
                nano_id: Some("defect-a-b".to_string()),
                timestamp: Some(1_000),
                raw: b"b".to_vec().into(),
                text_type: Some(1),
                send_user: peer_b_rbdc.clone(),
                recv_user: seed_uuid_rbdc.clone(),
            },
        )
        .await
        .context("插入缺陷A B 消息失败")?;
        ChatMessageRecord::insert(
            &test_rb,
            &ChatMessageRecord {
                id: None,
                session_uuid: session_ac.clone(),
                nano_id: Some("defect-a-c".to_string()),
                timestamp: Some(2_000),
                raw: b"c".to_vec().into(),
                text_type: Some(1),
                send_user: peer_c_rbdc.clone(),
                recv_user: seed_uuid_rbdc.clone(),
            },
        )
        .await
        .context("插入缺陷A C 消息失败")?;
        // 聚合建立两个会话的 user_session 行(与上线触发同源)
        aggregate_user_sessions(&test_rb, &seed_uuid_rbdc).await.context("聚合失败")?;
        // A 读取 C 会话
        let (status, json) = post_json(
            &app,
            "/msg/add_read_chat_record",
            Some(&bridge_read_body(&peer_c, "defect-a-c", 2_000, 1)),
            Some(&access_token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "C 会话上报应成功: {json}");
        // 拉未读: B 的消息必须仍返回, C 的已读不应返回
        let (status, json) =
            post_json(&app, "/msg/get_unread_chat_record", None, Some(&access_token)).await;
        assert_eq!(status, StatusCode::OK, "拉未读应成功: {json}");
        let nanos: Vec<String> = json["data"]
            .as_array()
            .map(|arr| {
                arr.iter().filter_map(|v| v["nano_id"].as_str().map(|s| s.to_string())).collect()
            })
            .unwrap_or_default();
        assert!(
            nanos.iter().any(|n| n == "defect-a-b"),
            "缺陷A: B 的未读不应因读取 C 会话而丢失: {json}"
        );
        assert!(!nanos.iter().any(|n| n == "defect-a-c"), "C 会话已读后不应再出现在未读: {json}");

        // 7.4 /session/read 钳制到 synced_id + /session/synced 推进
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
