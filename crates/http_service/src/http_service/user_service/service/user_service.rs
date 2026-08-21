use std::str::FromStr;

use anyhow::anyhow;
use common::config_str::{
    EMAIL_VERIFY_CODE, MOBILE_PLATFORM, PC_PLATFORM, REFRESH_TOKEN, REFRESH_TOKEN_PLATFORM,
};
use common::models::user_entity::basic_user::BasicUser;
use common::models::user_entity::email_sso::EmailSso;
use common::models::user_entity::user_info::UserInfo;
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
use crate::http_service::user_service::dto::refresh_token_dto::RefreshTokenDTO;
use crate::http_service::user_service::dto::sign_up_basic_user_dto::SignUpBasicUserDTO;
use crate::http_service::user_service::dto::update_user_dto::UpdateUserDTO;
use crate::http_service::user_service::vo::sign_in_vo::SignInResponseVO;
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
    // 1. 邮箱唯一性检查
    let normalized = normalize_email(email);
    if EmailSso::select_by_email_normalized(rb, &normalized).await?.is_some() {
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

pub async fn add_new_basic_user_service(
    rb: &RBatis,
    redis: &deadpool_redis::Pool,
    basic_user: SignUpBasicUserDTO,
) -> Result<String, anyhow::Error> {
    // 1. 邮箱唯一性检查
    let email = basic_user.email.clone().ok_or(anyhow!("邮箱为空"))?;
    let email_normalized = normalize_email(&email);
    if EmailSso::select_by_email_normalized(rb, &email_normalized).await?.is_some() {
        return Err(anyhow!("该邮箱已被注册"));
    }

    // 2. 校验注册验证码(与 Redis 中的一致,校验通过后删除)
    let code = basic_user.verification_code.as_ref().ok_or(anyhow!("验证码为空"))?;
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

    let mut basic_user = SignUpBasicUserDTO::to_basic_user(basic_user);
    basic_user.uuid = Some(Uuid::now_v7().to_string().parse()?);
    let password = basic_user.password.as_ref().ok_or(anyhow!("密码为空"))?;
    let hashed_password = hash_password(password)?;
    basic_user.password = Some(hashed_password);
    basic_user.icon = None;
    basic_user.info = Some("".to_string());

    let account_ref: &str = basic_user.account.as_deref().unwrap_or("");
    match get_exit_user(rb, account_ref).await {
        true => Err(anyhow!("该账号已存在!".to_string())),
        false => {
            let tx = rb.acquire_begin().await?;
            // 使用事务块包裹逻辑
            let result: Result<(), anyhow::Error> = async {
                let now = get_now_time_stamp_as_millis()?;
                let user_info = UserInfo {
                    uuid: basic_user.uuid.clone(),
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
                    uuid: basic_user.uuid.clone(),
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

            // 如果事务中有错误，回滚事务
            if result.is_err() {
                let _ = tx.rollback().await;
                return Err(anyhow!("事务执行错误"));
            }
            Ok(CommonResponseNoDataRef::success_empty())
        }
    }
}

/// 用户登录
pub async fn user_sign_in(
    rb: &RBatis,
    redis: &deadpool_redis::Pool,
    basic_user_dto: SignInBasicUserDTO,
) -> Result<String, anyhow::Error> {
    let platform =
        basic_user_dto.platform.as_ref().cloned().ok_or(anyhow!("平台为空".to_string()))?;
    if platform != PC_PLATFORM && platform != MOBILE_PLATFORM {
        return Err(anyhow!("暂不支持该平台登录".to_string()));
    }
    let basic_user = SignInBasicUserDTO::to_basic_user(basic_user_dto);
    let BasicUser { account, password, .. } = basic_user;

    let account_str = account.as_ref().ok_or(anyhow!("账号为空".to_string()))?;
    let password_str = password.as_ref().ok_or(anyhow!("密码为空".to_string()))?;

    let basic_user =
        BasicUser::select_by_account(rb, account_str).await?.ok_or(anyhow!("用户不存在"))?;

    let mut conn = redis.get().await?;

    let exit_password = basic_user.password.as_ref().ok_or(anyhow!("密码为空"))?;

    if verify_password(password_str, exit_password) {
        let uuid = basic_user.uuid.ok_or(anyhow!("账号为空"))?.to_string();
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

        let sign_in_vo = SignInResponseVO { access_token, refresh_token };
        Ok(CommonResponseRef::<SignInResponseVO>::success_json(&sign_in_vo)?)
    } else {
        Err(anyhow!("用户或密码不正确!"))
    }
}

/// 通过 refresh_token 换取短效 access_token
pub async fn refresh_access_token(
    redis: &deadpool_redis::Pool,
    refresh_token_dto: RefreshTokenDTO,
) -> Result<String, anyhow::Error> {
    let mut conn = redis.get().await?;

    let key = format!("{}{}", REFRESH_TOKEN, refresh_token_dto.refresh_token).to_uppercase();
    let result: RedisResult<String> = cmd("GET").arg(&key).query_async(&mut conn).await;
    let uuid = result.map_err(|_| anyhow!("refresh_token 无效或已过期"))?;

    let platform_key =
        format!("{}{}", REFRESH_TOKEN_PLATFORM, refresh_token_dto.refresh_token).to_uppercase();
    let platform: RedisResult<String> = cmd("GET").arg(&platform_key).query_async(&mut conn).await;
    let platform = platform.map_err(|_| anyhow!("无法获取平台信息"))?;

    // 生成新的短效 access_token (24h)
    let access_token = generate_access_token(uuid.clone(), platform.clone())?;
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

/// 验证用户传递的token
pub async fn verify_p2p_token_service(
    redis: &deadpool_redis::Pool,
    uuid: String,
    token: String,
    me: Option<String>,
) -> Result<String, anyhow::Error> {
    let mut conn = redis.get().await?;
    let me = me.ok_or(anyhow!("获取账号失败"))?;

    let key = format!("P2P:USER:AUTH:{}:{}", uuid, token).to_uppercase();
    let result: RedisResult<String> = cmd("GET").arg(&key).query_async(&mut conn).await;
    let res = result?;
    info!("结果: {} {}", uuid, res);
    match res == me {
        true => {
            let key = format!("{}{}", "USER_UDP_ADDRESS_", uuid).to_uppercase();
            let result: RedisResult<String> = cmd("GET").arg(&key).query_async(&mut conn).await;
            Ok(CommonResponseRef::<String>::success_json(&result?)?)
        }
        false => Err(anyhow!("failed")),
    }
}

/// 添加用户验证的token
pub async fn add_p2p_token_service(
    redis: &deadpool_redis::Pool,
    uuid: String,
    token: String,
    me: Option<String>,
) -> Result<String, anyhow::Error> {
    let mut conn = redis.get().await?;
    let me = me.ok_or(anyhow!("获取账号失败"))?;

    let key = format!("P2P:USER:AUTH:{}:{}", me, token).to_uppercase();
    let _: () = cmd("SET")
        .arg(&key)
        .arg(uuid.to_string())
        .arg("EX")
        .arg(600)
        .query_async(&mut conn)
        .await?;
    Ok(CommonResponseNoDataRef::success_empty())
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
