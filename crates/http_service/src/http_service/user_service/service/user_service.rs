use std::str::FromStr;

use actix_web::HttpRequest;
use actix_web::http::header::USER_AGENT;
use anyhow::anyhow;
use common::config_str::{
    EMAIL_VERIFY_CODE, MOBILE_PLATFORM, PC_PLATFORM, REFRESH_TOKEN, REFRESH_TOKEN_DEVICE,
    REFRESH_TOKEN_PLATFORM, REGISTER_SESSION_TOKEN,
};
use common::models::user_entity::basic_user::BasicUser;
use common::models::user_entity::email_sso::EmailSso;
use common::models::user_entity::user_info::UserInfo;
use common::models::user_entity::user_login_log::{
    LOGIN_EVENT_ACCOUNT_NOT_FOUND, LOGIN_EVENT_PASSWORD_FAIL, LOGIN_EVENT_REFRESH,
    LOGIN_EVENT_SUCCESS, LOGIN_TYPE_ACCOUNT, LOGIN_TYPE_REFRESH, UserLoginLog,
};
use common::utils::jwt_util::{generate_access_token, generate_token_with_expiry};
use common::utils::rsa_util::{hash_password, verify_password};
use common::utils::time::get_now_time_stamp_as_millis;
use common::utils::validators::normalize_email;
use deadpool_redis::redis::{AsyncCommands, RedisResult, cmd};
use email_service::manager::EmailManager;
use email_service::{Email, EmailAddress};
use rand::Rng;
use rbatis::{RBatis, rbdc};
use rbs::value;
use tracing::{error, info};
use uuid::Uuid;

use crate::http_service::user_service::dto::basic_user_dto::SignInBasicUserDTO;
use crate::http_service::user_service::dto::complete_profile_dto::CompleteProfileDTO;
use crate::http_service::user_service::dto::refresh_token_dto::RefreshTokenDTO;
use crate::http_service::user_service::dto::sign_up_step1_dto::SignUpStep1DTO;
use crate::http_service::user_service::dto::update_user_dto::UpdateUserDTO;
use crate::http_service::user_service::vo::sign_in_vo::SignInResponseVO;
use crate::http_service::user_service::vo::sign_up_step1_vo::SignUpStep1ResponseVO;
use crate::http_service::user_service::vo::user_info::UserInfoVO;
use crate::utils::http_response::{CommonResponseNoDataRef, CommonResponseRef};

pub async fn test_sql(rb: &RBatis) -> Vec<BasicUser> {
    let basic_user_all = match BasicUser::select_all(rb).await {
        Ok(v) => v,
        Err(e) => {
            error!("select_all 查询错误: {}", e);
            vec![]
        }
    };
    let basic_user_icon = match BasicUser::select_by_map(rb, value! { "icon": "33333" }).await {
        Ok(v) => v,
        Err(e) => {
            error!("select_by_map 查询错误: {}", e);
            vec![]
        }
    };
    info!("1 {:?}", basic_user_all);
    info!("2 {:?}", basic_user_icon);
    basic_user_all
}

pub async fn get_exit_user(rb: &RBatis, account: &str) -> bool {
    match BasicUser::select_by_account(rb, account).await {
        Ok(user) => user.is_some(),
        Err(error) => {
            error!("查询用户是否存在失败: {}", error);
            true
        }
    }
}

/// 发送注册邮箱验证码到指定邮箱,验证码写入 Redis 5 分钟有效
pub async fn send_verify_code_service(
    rb: &RBatis,
    redis: &deadpool_redis::Pool,
    email_manager: &EmailManager,
    email: &str,
) -> Result<String, anyhow::Error> {
    // 1. 邮箱唯一性检查(仅已完成注册的邮箱视为已占用; 占位未完成允许重新获取验证码)
    let normalized = normalize_email(email);
    if email_already_registered(rb, &normalized).await? {
        return Err(anyhow!("该邮箱已被注册"));
    }

    // 2. 生成 6 位数字验证码
    let code = format!("{:06}", rand::thread_rng().gen_range(0..1_000_000));

    // 3. 写入 Redis,5 分钟(300 秒)过期
    let mut conn = redis.get().await?;
    let key = format!("{}{}", EMAIL_VERIFY_CODE, email).to_uppercase();
    conn.set_ex::<&str, &str, ()>(&key, &code, 300).await?;
    info!("验证码已存储,邮箱: {}", email);

    // 4. 通过阿里云邮件发送验证码
    let account_name = common::config_manager::get_config("email.account_name").unwrap_or_default();
    let mail = Email::builder()
        .from(EmailAddress::new(&account_name).map_err(|e| anyhow!("发件人配置错误: {}", e))?)
        .to(EmailAddress::new(email).map_err(|e| anyhow!("收件人邮箱格式错误: {}", e))?)
        .subject("OnlyTalk 注册验证码")
        .text_body(format!("您的注册验证码是: {},5 分钟内有效,请勿泄露给他人。", code))
        .build()
        .map_err(|e| anyhow!("构建邮件失败: {}", e))?;

    let result = email_manager.send(&mail).await.map_err(|e| anyhow!("邮件发送失败: {}", e))?;
    if !result.is_success() {
        let reason = result.error.as_ref().map(|e| e.message.clone()).unwrap_or_default();
        return Err(anyhow!("邮件发送失败: {}", reason));
    }

    Ok(CommonResponseNoDataRef::success_empty())
}

/// 邮箱是否已被完成注册。
/// 返回 true 表示该邮箱已被占(已完成注册用户占用); 占位未完成(registration_status=0)视为可重新注册。
async fn email_already_registered(
    rb: &RBatis,
    email_normalized: &str,
) -> Result<bool, anyhow::Error> {
    let Some(sso) = EmailSso::select_by_email_normalized(rb, email_normalized).await? else {
        return Ok(false);
    };
    let Some(uuid) = sso.uuid else {
        // 理论不应发生(外键必指向 basic_user), 保守视为已占用
        return Ok(true);
    };
    let user = BasicUser::select_by_uuid(rb, &uuid).await?;
    Ok(match user {
        Some(u) => u.registration_status == Some(1),
        None => true,
    })
}

/// 两步注册第一步: 校验邮箱 + 验证码, 创建占位用户并下发注册会话 token
pub async fn sign_up_step1_service(
    rb: &RBatis,
    redis: &deadpool_redis::Pool,
    dto: SignUpStep1DTO,
) -> Result<String, anyhow::Error> {
    // 1. 邮箱检查: 已完成注册则拒绝; 已有占位用户(注册未完成)则允许复用继续注册
    let email = dto.email.clone().ok_or(anyhow!("邮箱为空"))?;
    let email_normalized = normalize_email(&email);
    if email_already_registered(rb, &email_normalized).await? {
        return Err(anyhow!("该邮箱已被注册"));
    }
    let existing_placeholder =
        match EmailSso::select_by_email_normalized(rb, &email_normalized).await? {
            Some(sso) => match sso.uuid {
                Some(uuid) => BasicUser::select_by_uuid(rb, &uuid)
                    .await?
                    .filter(|u| u.registration_status == Some(0)),
                None => None,
            },
            None => None,
        };

    // 2. 校验注册验证码(与 Redis 中的一致,校验通过后删除)
    let code = dto.verification_code.as_ref().ok_or(anyhow!("验证码为空"))?;
    let mut conn = redis.get().await?;
    let code_key = format!("{}{}", EMAIL_VERIFY_CODE, email_normalized).to_uppercase();
    let stored: Option<String> = conn.get(&code_key).await?;
    match stored {
        Some(stored) if stored == *code => {
            let _: Result<(), _> = conn.del(&code_key).await;
        }
        _ => {
            return Err(anyhow!("验证码错误或已过期"));
        }
    }

    // 3. 复用已有占位用户, 否则创建新占位用户(registration_status=0 未完成,不可登录)
    let uuid: rbdc::Uuid = if let Some(placeholder) = existing_placeholder {
        placeholder.uuid.clone().ok_or(anyhow!("占位用户缺少 uuid"))?
    } else {
        let new_uuid: rbdc::Uuid = Uuid::now_v7().to_string().parse()?;
        let tx = rb.acquire_begin().await?;
        let result: Result<(), anyhow::Error> = async {
            let now = get_now_time_stamp_as_millis()?;
            let placeholder_account = format!("u_{}", new_uuid);
            let placeholder_password = hash_password(&Uuid::now_v7().to_string())?;

            let basic_user = BasicUser {
                uuid: Some(new_uuid.clone()),
                username: Some("".to_string()),
                account: Some(placeholder_account),
                icon: None,
                info: Some("".to_string()),
                password: Some(placeholder_password),
                registration_status: Some(0),
            };

            let user_info = UserInfo {
                uuid: Some(new_uuid.clone()),
                gender: None,
                age: Some(0),
                birthday: Some(0),
                note: Some("这个人很勤快，但什么都没写".to_string()),
                created_at: Some(now),
                updated_at: Some(now),
                phone: None,
                email: Some(email.clone()),
                address: None,
                status: None,
            };

            let email_sso = EmailSso {
                uuid: Some(new_uuid.clone()),
                email: Some(email.clone()),
                email_normalized: Some(email_normalized.clone()),
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
            };

            BasicUser::insert(&tx, &basic_user).await?;
            UserInfo::insert(&tx, &user_info).await?;
            EmailSso::insert(&tx, &email_sso).await?;

            tx.commit().await?;
            Ok(())
        }
        .await;

        if result.is_err() {
            let _ = tx.rollback().await;
            return Err(anyhow!("创建占位用户失败"));
        }
        new_uuid
    };

    // 4. 生成注册会话 token 并写入 Redis(映射 uuid, 30 分钟有效)
    let reg_token = Uuid::now_v7().to_string();
    let key = format!("{}{}", REGISTER_SESSION_TOKEN, reg_token).to_uppercase();
    let mut conn = redis.get().await?;
    conn.set_ex::<&str, &str, ()>(&key, &uuid.to_string(), 1800).await?;

    let vo = SignUpStep1ResponseVO { reg_token, uuid: uuid.to_string() };
    Ok(CommonResponseRef::<SignUpStep1ResponseVO>::success_json(&vo)?)
}

/// 两步注册第二步: 凭注册会话 token 补全账号、用户名与密码, 完成注册
pub async fn complete_profile_service(
    rb: &RBatis,
    redis: &deadpool_redis::Pool,
    dto: CompleteProfileDTO,
) -> Result<String, anyhow::Error> {
    // 1. 校验注册会话 token, 获取占位用户 uuid
    let reg_token = dto.reg_token.as_ref().ok_or(anyhow!("注册会话token为空"))?;
    let mut conn = redis.get().await?;
    let key = format!("{}{}", REGISTER_SESSION_TOKEN, reg_token).to_uppercase();
    let uuid_str: Option<String> = conn.get(&key).await?;
    let uuid_str = uuid_str.ok_or(anyhow!("注册会话已失效或不存在"))?;
    let uuid: rbdc::Uuid = uuid_str.parse().map_err(|_| anyhow!("注册会话无效"))?;

    // 1.5 校验邮箱归属: 防止 reg_token 被冒用/重放, 提交的邮箱必须与占位用户一致
    let email = dto.email.as_ref().ok_or(anyhow!("邮箱为空"))?;
    let email_normalized = normalize_email(email);
    let sso = EmailSso::select_by_uuid(rb, &uuid).await?.ok_or(anyhow!("邮箱渠道不存在"))?;
    if sso.email_normalized.as_deref() != Some(email_normalized.as_str()) {
        return Err(anyhow!("邮箱与注册会话不匹配"));
    }

    // 2. 校验账号唯一性
    let account = dto.account.as_ref().ok_or(anyhow!("账号为空"))?;
    if BasicUser::select_by_account(rb, account).await?.is_some() {
        return Err(anyhow!("该账号已存在"));
    }

    // 3. 校验密码并哈希
    let password = dto.password.as_ref().ok_or(anyhow!("密码为空"))?;
    let hashed_password = hash_password(password)?;

    // 4. 补全占位用户信息(账号/用户名/密码/简介, registration_status=1 完成注册)
    let mut basic_user =
        BasicUser::select_by_uuid(rb, &uuid).await?.ok_or(anyhow!("用户不存在"))?;
    basic_user.account = Some(account.clone());
    basic_user.password = Some(hashed_password);
    basic_user.username = Some(dto.username.clone().unwrap_or_default());
    if let Some(ref info) = dto.info {
        basic_user.info = Some(info.clone());
    }
    if let Some(ref icon) = dto.icon {
        basic_user.icon = Some(icon.clone());
    }
    basic_user.registration_status = Some(1);
    BasicUser::update_by_uuid(rb, &basic_user, &uuid).await?;

    // 5. 消费注册会话 token
    let _: Result<(), _> = conn.del(&key).await;

    Ok(CommonResponseNoDataRef::success_empty())
}

/// 从 HttpRequest 提取客户端 IP(ipv4/ipv6)与 User-Agent
fn extract_client_info(req: &HttpRequest) -> (Option<String>, Option<String>, Option<String>) {
    let (ipv4, ipv6) = match req.peer_addr() {
        Some(addr) if addr.ip().is_ipv4() => (Some(addr.ip().to_string()), None),
        Some(addr) if addr.ip().is_ipv6() => (None, Some(addr.ip().to_string())),
        _ => (None, None),
    };
    let user_agent =
        req.headers().get(USER_AGENT).and_then(|v| v.to_str().ok()).map(|s| s.to_string());
    (ipv4, ipv6, user_agent)
}

/// 写入登录审计记录（审计失败仅记录日志，不阻塞登录主流程）
async fn write_login_log(rb: &RBatis, log: UserLoginLog) {
    if let Err(e) = UserLoginLog::insert(rb, &log).await {
        error!("写入登录审计记录失败: {}", e);
    }
}

/// 用户登录
pub async fn user_sign_in(
    rb: &RBatis,
    redis: &deadpool_redis::Pool,
    basic_user_dto: SignInBasicUserDTO,
    req: &HttpRequest,
) -> Result<String, anyhow::Error> {
    let platform =
        basic_user_dto.platform.as_ref().cloned().ok_or(anyhow!("平台为空".to_string()))?;
    if platform != PC_PLATFORM && platform != MOBILE_PLATFORM {
        return Err(anyhow!("暂不支持该平台登录".to_string()));
    }
    let device_fingerprint =
        basic_user_dto.device_fingerprint.clone().ok_or(anyhow!("设备指纹为空".to_string()))?;
    let basic_user = SignInBasicUserDTO::to_basic_user(basic_user_dto);
    let BasicUser { account, password, .. } = basic_user;

    let account_str = account.as_ref().ok_or(anyhow!("账号为空".to_string()))?;
    let password_str = password.as_ref().ok_or(anyhow!("密码为空".to_string()))?;

    let (ipv4, ipv6, user_agent) = extract_client_info(req);

    let basic_user = match BasicUser::select_by_account(rb, account_str).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            // 账号不存在: 记录审计,保留提交的账号便于追踪
            write_login_log(
                rb,
                UserLoginLog {
                    id: None,
                    uuid: None,
                    account: Some(account_str.to_string()),
                    login_type: Some(LOGIN_TYPE_ACCOUNT.to_string()),
                    event_type: Some(LOGIN_EVENT_ACCOUNT_NOT_FOUND.to_string()),
                    login_at: get_now_time_stamp_as_millis().ok(),
                    platform: Some(platform.clone()),
                    ipv4: ipv4.clone(),
                    ipv6: ipv6.clone(),
                    user_agent: user_agent.clone(),
                    device: Some(device_fingerprint.clone()),
                    result: None,
                },
            )
            .await;
            return Err(anyhow!("用户不存在"));
        }
        Err(e) => return Err(anyhow!("查询用户失败: {}", e)),
    };

    // 占位未完成注册的用户不允许登录
    if basic_user.registration_status != Some(1) {
        return Err(anyhow!("该账号尚未完成注册,请先补全资料"));
    }

    let mut conn = redis.get().await?;

    let exit_password = basic_user.password.as_ref().ok_or(anyhow!("密码为空"))?;

    if verify_password(password_str, exit_password) {
        let uuid = basic_user.uuid.clone().ok_or(anyhow!("账号为空"))?.to_string();
        // 短效 token (24h)
        let access_token = generate_access_token(uuid.clone(), platform.clone())?;
        // 长效 refresh token (30 days)
        let refresh_token =
            generate_token_with_expiry(uuid.clone(), platform.clone(), 3600 * 24 * 30)?;

        // 存储 refresh_token 到 Redis (30 天过期)
        let rt_key = format!("{}{}", REFRESH_TOKEN, refresh_token).to_uppercase();
        let _: () = cmd("SET")
            .arg(&rt_key)
            .arg(&uuid)
            .arg("EX")
            .arg(3600 * 24 * 30)
            .query_async(&mut conn)
            .await?;
        let rt_platform_key = format!("{}{}", REFRESH_TOKEN_PLATFORM, refresh_token).to_uppercase();
        let _: () = cmd("SET")
            .arg(&rt_platform_key)
            .arg(&platform)
            .arg("EX")
            .arg(3600 * 24 * 30)
            .query_async(&mut conn)
            .await?;
        // 存储 refresh_token 绑定的设备指纹 (30 天过期)
        let rt_device_key = format!("{}{}", REFRESH_TOKEN_DEVICE, refresh_token).to_uppercase();
        let _: () = cmd("SET")
            .arg(&rt_device_key)
            .arg(&device_fingerprint)
            .arg("EX")
            .arg(3600 * 24 * 30)
            .query_async(&mut conn)
            .await?;

        // 登录成功审计
        write_login_log(
            rb,
            UserLoginLog {
                id: None,
                uuid: basic_user.uuid.clone(),
                account: Some(account_str.to_string()),
                login_type: Some(LOGIN_TYPE_ACCOUNT.to_string()),
                event_type: Some(LOGIN_EVENT_SUCCESS.to_string()),
                login_at: get_now_time_stamp_as_millis().ok(),
                platform: Some(platform.clone()),
                ipv4,
                ipv6,
                user_agent,
                device: Some(device_fingerprint),
                result: None,
            },
        )
        .await;

        let sign_in_vo = SignInResponseVO { access_token, refresh_token };
        Ok(CommonResponseRef::<SignInResponseVO>::success_json(&sign_in_vo)?)
    } else {
        // 密码错误审计
        write_login_log(
            rb,
            UserLoginLog {
                id: None,
                uuid: basic_user.uuid.clone(),
                account: Some(account_str.to_string()),
                login_type: Some(LOGIN_TYPE_ACCOUNT.to_string()),
                event_type: Some(LOGIN_EVENT_PASSWORD_FAIL.to_string()),
                login_at: get_now_time_stamp_as_millis().ok(),
                platform: Some(platform.clone()),
                ipv4: ipv4.clone(),
                ipv6: ipv6.clone(),
                user_agent: user_agent.clone(),
                device: Some(device_fingerprint),
                result: None,
            },
        )
        .await;
        Err(anyhow!("用户或密码不正确!"))
    }
}

/// 通过 refresh_token 换取短效 access_token
pub async fn refresh_access_token(
    rb: &RBatis,
    redis: &deadpool_redis::Pool,
    refresh_token_dto: RefreshTokenDTO,
    req: &HttpRequest,
) -> Result<String, anyhow::Error> {
    let mut conn = redis.get().await?;

    let (ipv4, ipv6, user_agent) = extract_client_info(req);

    let key = format!("{}{}", REFRESH_TOKEN, refresh_token_dto.refresh_token).to_uppercase();
    let result: RedisResult<String> = cmd("GET").arg(&key).query_async(&mut conn).await;
    let uuid = match result {
        Ok(u) => u,
        Err(e) => {
            // token 无效/过期审计(uuid 无法解析,留空;设备指纹记录客户端提交值)
            write_login_log(
                rb,
                UserLoginLog {
                    id: None,
                    uuid: None,
                    account: None,
                    login_type: Some(LOGIN_TYPE_REFRESH.to_string()),
                    event_type: Some(LOGIN_EVENT_REFRESH.to_string()),
                    login_at: get_now_time_stamp_as_millis().ok(),
                    platform: None,
                    ipv4: ipv4.clone(),
                    ipv6: ipv6.clone(),
                    user_agent: user_agent.clone(),
                    device: Some(refresh_token_dto.device_fingerprint.clone()),
                    result: Some("refresh_token 无效或已过期".to_string()),
                },
            )
            .await;
            return Err(anyhow!("refresh_token 无效或已过期: {}", e));
        }
    };

    let platform_key =
        format!("{}{}", REFRESH_TOKEN_PLATFORM, refresh_token_dto.refresh_token).to_uppercase();
    let platform: RedisResult<String> = cmd("GET").arg(&platform_key).query_async(&mut conn).await;
    let platform = platform.map_err(|_| anyhow!("无法获取平台信息"))?;

    // 校验设备指纹: refresh_token 必须与其绑定的设备指纹一致
    let device_key =
        format!("{}{}", REFRESH_TOKEN_DEVICE, refresh_token_dto.refresh_token).to_uppercase();
    let stored_device: RedisResult<String> =
        cmd("GET").arg(&device_key).query_async(&mut conn).await;
    if !device_fingerprint_matches(
        stored_device.as_deref().ok(),
        &refresh_token_dto.device_fingerprint,
    ) {
        write_login_log(
            rb,
            UserLoginLog {
                id: None,
                uuid: uuid.parse::<rbatis::rbdc::Uuid>().ok(),
                account: None,
                login_type: Some(LOGIN_TYPE_REFRESH.to_string()),
                event_type: Some(LOGIN_EVENT_REFRESH.to_string()),
                login_at: get_now_time_stamp_as_millis().ok(),
                platform: Some(platform.clone()),
                ipv4: ipv4.clone(),
                ipv6: ipv6.clone(),
                user_agent: user_agent.clone(),
                device: Some(refresh_token_dto.device_fingerprint.clone()),
                result: Some("设备指纹不匹配".to_string()),
            },
        )
        .await;
        return Err(anyhow!("设备不匹配，请重新登录"));
    }

    // 生成新的短效 access_token (24h)
    let access_token = generate_access_token(uuid.clone(), platform.clone())?;

    // 刷新成功审计
    write_login_log(
        rb,
        UserLoginLog {
            id: None,
            uuid: uuid.parse::<rbatis::rbdc::Uuid>().ok(),
            account: None,
            login_type: Some(LOGIN_TYPE_REFRESH.to_string()),
            event_type: Some(LOGIN_EVENT_REFRESH.to_string()),
            login_at: get_now_time_stamp_as_millis().ok(),
            platform: Some(platform.clone()),
            ipv4,
            ipv6,
            user_agent,
            device: Some(refresh_token_dto.device_fingerprint),
            result: None,
        },
    )
    .await;

    let sign_in_vo =
        SignInResponseVO { access_token, refresh_token: refresh_token_dto.refresh_token.clone() };
    Ok(CommonResponseRef::<SignInResponseVO>::success_json(&sign_in_vo)?)
}

pub async fn get_user_info_by_account(
    rbatis: &RBatis,
    account: Option<String>,
) -> Result<String, anyhow::Error> {
    let account = account.ok_or(anyhow!("账号为空"))?;

    let basic_user =
        BasicUser::select_by_account(rbatis, &account).await?.ok_or(anyhow!("查询为空"))?;
    let uuid = basic_user.uuid.as_ref().ok_or(anyhow!("账号id为空"))?;
    let user_info = UserInfo::select_by_uuid(rbatis, uuid).await?.ok_or(anyhow!("查询为空"))?;
    let user_info_vo = UserInfoVO::from((user_info, basic_user));
    Ok(CommonResponseRef::<UserInfoVO>::success_json(&user_info_vo)?)
}

pub async fn get_user_info_by_uuid(
    rbatis: &RBatis,
    uuid: Option<String>,
) -> Result<String, anyhow::Error> {
    let uuid = uuid.ok_or(anyhow!("账号为空"))?;
    let uuid = rbatis::rbdc::Uuid::from_str(uuid.as_str())?;

    let basic_user = BasicUser::select_by_uuid(rbatis, &uuid).await?.ok_or(anyhow!("查询为空"))?;
    let uuid = basic_user.uuid.as_ref().ok_or(anyhow!("账号id为空"))?;
    let user_info = UserInfo::select_by_uuid(rbatis, uuid).await?.ok_or(anyhow!("查询为空"))?;
    let user_info_vo = UserInfoVO::from((user_info, basic_user));
    Ok(CommonResponseRef::<UserInfoVO>::success_json(&user_info_vo)?)
}

/// 获取用户的uuid
pub async fn get_user_uuid_by_account_service(
    rb: &RBatis,
    redis: &deadpool_redis::Pool,
    account: String,
) -> Result<String, anyhow::Error> {
    let result = get_user_uuid_by_account(rb, redis, account).await?;
    Ok(CommonResponseRef::<String>::success_json(&result.to_string())?)
}

/// 获取用户的uuid
pub async fn get_user_uuid_by_account(
    rb: &RBatis,
    redis: &deadpool_redis::Pool,
    account: String,
) -> Result<Uuid, anyhow::Error> {
    let key = format!("{}{}", "USER_UUID_", account);
    let key = key.to_uppercase();
    let mut conn = redis.get().await?;

    let result: RedisResult<String> = cmd("GET").arg(&key).query_async(&mut conn).await;
    let uuid = match result {
        Ok(v) => return Ok(Uuid::parse_str(v.as_str())?),
        Err(_) => {
            let basic_user =
                BasicUser::select_by_account(rb, &account).await?.ok_or(anyhow!("账号不存在"))?;
            basic_user.uuid.ok_or(anyhow!("账号id为空"))?
        }
    };
    // 设置24小时的缓存
    let _: () = cmd("SET")
        .arg(&key)
        .arg(uuid.to_string())
        .arg("EX")
        .arg(86400)
        .query_async(&mut conn)
        .await?;
    Ok(uuid.to_string().parse()?)
}

pub async fn update_user_avatar(
    rb: &RBatis,
    biz_id: String,
    user_id: rbdc::types::uuid::Uuid,
) -> Result<(), anyhow::Error> {
    let mut basic_user =
        BasicUser::select_by_uuid(rb, &user_id).await?.ok_or(anyhow!("用户不存在"))?;
    basic_user.icon = Some(biz_id);
    BasicUser::update_by_map(rb, &basic_user, value! { "uuid": &user_id }).await?;

    Ok(())
}

pub async fn update_user_info_service(
    rb: &RBatis,
    uuid: Option<String>,
    update_dto: UpdateUserDTO,
) -> Result<String, anyhow::Error> {
    let uuid_str = uuid.ok_or(anyhow!("用户ID为空"))?;
    let uuid = rbatis::rbdc::Uuid::from_str(&uuid_str)?;

    let mut basic_user =
        BasicUser::select_by_uuid(rb, &uuid).await?.ok_or(anyhow!("用户不存在"))?;
    let mut user_info =
        UserInfo::select_by_uuid(rb, &uuid).await?.ok_or(anyhow!("用户详情不存在"))?;

    update_dto.apply_to_basic_user(&mut basic_user);
    update_dto.apply_to_user_info(&mut user_info)?;

    let tx = rb.acquire_begin().await?;
    let result: Result<(), anyhow::Error> = async {
        BasicUser::update_by_map(&tx, &basic_user, value! { "uuid": &uuid }).await?;
        UserInfo::update_by_uuid(&tx, &user_info, &uuid).await?;
        tx.commit().await?;
        Ok(())
    }
    .await;

    if result.is_err() {
        let _ = tx.rollback().await;
        return Err(anyhow!("更新用户信息失败"));
    }

    Ok(CommonResponseNoDataRef::success_empty())
}

/// 校验 refresh_token 绑定的设备指纹是否与请求携带的一致
fn device_fingerprint_matches(stored: Option<&str>, provided: &str) -> bool {
    stored.is_some_and(|s| s == provided)
}

#[cfg(test)]
mod tests {
    use super::device_fingerprint_matches;

    #[test]
    fn device_fingerprint_matches_identical() {
        assert!(device_fingerprint_matches(Some("fp-1"), "fp-1"));
    }

    #[test]
    fn device_fingerprint_matches_different() {
        assert!(!device_fingerprint_matches(Some("fp-1"), "fp-2"));
    }

    #[test]
    fn device_fingerprint_matches_missing_binding() {
        // 旧 token 没有设备绑定(Redis 无 DEVICE key),应视为不匹配
        assert!(!device_fingerprint_matches(None, "fp-1"));
    }
}
