use std::str::FromStr;

use anyhow::anyhow;
use deadpool_redis::redis::{RedisResult, cmd};
use common::config_str::{APP_DOMAIN, MOBILE_PLATFORM, PC_PLATFORM, USER_DEFAULT_ICON, USER_FILE_PUBLIC};
use common::models::user_entity::basic_user::BasicUser;
use common::models::user_entity::basic_user_salt::{BasicUserSalt, get_user_salt};
use common::models::user_entity::user_info::UserInfo;
use common::utils::jwt_util::{get_jwt, get_jwt_with_expiry};
use common::utils::redis_utils::get_redis_conn;
use common::utils::rsa_util::{generate_random_string, hash_with_salt};
use common::utils::time::get_now_time_stamp_as_millis;
use common::{RBATIS_DATABASE, REDIS_CLIENT};
use tracing::{error, info};
use rbatis::{RBatis, rbdc};
use rbs::value;
use uuid::Uuid;

use crate::http_service::user_service::dto::basic_user_dto::SignInBasicUserDTO;
use crate::http_service::user_service::dto::refresh_token_dto::RefreshTokenDTO;
use crate::http_service::user_service::dto::sign_up_basic_user_dto::SignUpBasicUserDTO;
use crate::http_service::user_service::dto::update_user_dto::UpdateUserDTO;
use crate::http_service::user_service::vo::sign_in_vo::SignInResponseVO;
use crate::http_service::user_service::vo::user_info::UserInfoVO;
use crate::utils::http_response::{CommonResponseNoDataRef, CommonResponseRef};

pub async fn test_sql(rb: &RBatis) -> Vec<BasicUser> {
    let basic_user_all = BasicUser::select_all(rb).await.expect("查询出错");
    let basic_user_icon = BasicUser::select_by_map(rb, value! { "icon": "33333" }).await.expect("查询出错");
    let basic_user_all_id = BasicUser::select_all_by_id(rb, "33333", "4444444").await.expect("查询出错");
    info!("1 {:?}", basic_user_all);
    info!("2 {:?}", basic_user_icon);
    info!("3 {:?}", basic_user_all_id);
    basic_user_all
}

pub async fn get_exit_user(rb: &RBatis, account: &str) -> bool {
    match BasicUser::select_by_account(rb, account).await {
        Ok(user) => user.is_some(),
        Err(error) => {
            error!("查询用户是否存在出错 {}", error);
            true
        }
    }
}

pub async fn add_new_basic_user_service(
    rb: &RBatis,
    basic_user: SignUpBasicUserDTO,
) -> Result<String, anyhow::Error> {
    let mut basic_user = SignUpBasicUserDTO::to_basic_user(basic_user);
    basic_user.uuid = Some(Uuid::now_v7().to_string().parse()?);
    let random_str = generate_random_string(16);
    let password = hash_with_salt(basic_user.password.as_ref().ok_or(anyhow!("密码为空"))?, &random_str);
    basic_user.password = Option::from(password);
    let icon_url = format!("{}{}/{}", APP_DOMAIN, USER_FILE_PUBLIC, USER_DEFAULT_ICON);
    basic_user.icon = Some(icon_url);
    basic_user.info = Some("".to_string());

    let account_ref: &str = basic_user.account.as_deref().unwrap_or("");
    match get_exit_user(rb, account_ref).await {
        true => Err(anyhow!("该账号已存在!".to_string())),
        false => {
            let tx = rb.acquire_begin().await?;
            // 使用事务块包裹逻辑
            let result: Result<(), anyhow::Error> = async {
                let now = get_now_time_stamp_as_millis()?;
                let basic_user_salt = BasicUserSalt {
                    uuid: basic_user.uuid.clone(),
                    sign_up_salt: Option::from(random_str),
                };

                let user_info = UserInfo {
                    uuid: basic_user.uuid.clone(),
                    gender: None,
                    age: Some(0),
                    birthday: Some(0),
                    note: Some("这个人很勤快，但什么都没写".to_string()),
                    created_at: Some(now),
                    updated_at: Some(now),
                    phone: None,
                    email: None,
                    address: None,
                    status: None,
                };

                BasicUserSalt::insert(&tx, &basic_user_salt).await?;
                BasicUser::insert(&tx, &basic_user).await?;
                UserInfo::insert(&tx, &user_info).await?;

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
    basic_user_dto: SignInBasicUserDTO,
) -> Result<String, anyhow::Error> {
    let platform = basic_user_dto.platform.as_ref().cloned().ok_or(anyhow!("平台为空".to_string()))?;
    if platform != PC_PLATFORM && platform != MOBILE_PLATFORM {
        return Err(anyhow!("暂不支持该平台登录".to_string()))
    }
    let basic_user = SignInBasicUserDTO::to_basic_user(basic_user_dto);
    let BasicUser { account, password, .. } = basic_user;

    let account_str = account.as_ref().ok_or(anyhow!("账号为空".to_string()))?;
    let password_str = password.as_ref().ok_or(anyhow!("密码为空".to_string()))?;

    let basic_user =
        BasicUser::select_by_account(rb, account_str).await?.ok_or(anyhow!("用户不存在"))?;

    let mut conn = get_redis_conn().await?;
    let salt = get_user_salt(rb, &basic_user.uuid).await?;

    let hashed_password = hash_with_salt(
        password_str,
        salt.sign_up_salt.as_ref().ok_or(anyhow!("加密盐查询失败".to_string()))?,
    );

    let exit_password = basic_user.password.as_ref().ok_or(anyhow!("账号为空"))?;

    if exit_password == &hashed_password {
        let uuid = basic_user.uuid.ok_or(anyhow!("账号为空"))?.to_string();
        // 短效 token (24h)
        let access_token = get_jwt(uuid.clone(), platform.clone())?;
        // 长效 refresh token (30 days)
        let refresh_token = get_jwt_with_expiry(uuid.clone(), platform.clone(), 3600 * 24 * 30)?;

        // 存储 refresh_token 到 Redis (30 天过期)
        let rt_key = format!("REFRESH_TOKEN:{}", refresh_token);
        let _: () = cmd("SET").arg(&rt_key).arg(&uuid).arg("EX").arg(3600 * 24 * 30).query_async(&mut conn).await?;
        let rt_platform_key = format!("REFRESH_TOKEN:PLATFORM:{}", refresh_token);
        let _: () = cmd("SET").arg(&rt_platform_key).arg(&platform).arg("EX").arg(3600 * 24 * 30).query_async(&mut conn).await?;

        let sign_in_vo = SignInResponseVO { access_token, refresh_token };
        Ok(CommonResponseRef::<SignInResponseVO>::success_json(&sign_in_vo)?)
    } else {
        Err(anyhow!("用户或密码不正确!"))
    }
}

/// 通过 refresh_token 换取短效 access_token
pub async fn refresh_access_token(
    refresh_token_dto: RefreshTokenDTO,
) -> Result<String, anyhow::Error> {
    let redis_client = REDIS_CLIENT.read().await;
    let redis_conn = redis_client.as_ref().ok_or(anyhow!("redis客户端错误"))?;
    let mut conn = redis_conn.get().await?;

    let key = format!("REFRESH_TOKEN:{}", refresh_token_dto.refresh_token);
    let result: RedisResult<String> = cmd("GET").arg(&key).query_async(&mut conn).await;
    let uuid = result.map_err(|_| anyhow!("refresh_token 无效或已过期"))?;

    let platform_key = format!("REFRESH_TOKEN:PLATFORM:{}", refresh_token_dto.refresh_token);
    let platform: RedisResult<String> = cmd("GET").arg(&platform_key).query_async(&mut conn).await;
    let platform = platform.map_err(|_| anyhow!("无法获取平台信息"))?;

    // 生成新的短效 access_token (24h)
    let access_token = get_jwt(uuid.clone(), platform.clone())?;
    let sign_in_vo = SignInResponseVO { access_token, refresh_token: refresh_token_dto.refresh_token.clone() };
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
pub async fn get_user_uuid_by_account_service(account: String) -> Result<String, anyhow::Error> {
    let result = get_user_uuid_by_account(account).await?;
    Ok(CommonResponseRef::<String>::success_json(&result.to_string())?)
}

/// 获取用户的uuid
pub async fn get_user_uuid_by_account(account: String) -> Result<Uuid, anyhow::Error> {
    let rb = RBATIS_DATABASE.read().await;
    let rb = rb.as_ref().ok_or(anyhow!("获取连接失败"))?;

    let key = format!("{}{}", "USER_UUID_", account);
    let key = key.to_uppercase();
    let redis_client = REDIS_CLIENT.read().await;
    let redis_conn = redis_client.as_ref().ok_or(anyhow!("redis客户端错误"))?;
    let mut conn = redis_conn.get().await?;

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
    uuid: String,
    token: String,
    me: Option<String>,
) -> Result<String, anyhow::Error> {
    let mut conn = {
        let redis_client = REDIS_CLIENT.read().await;
        let redis_conn = redis_client.as_ref().ok_or(anyhow!("redis客户端错误"))?;
        redis_conn.get().await?
    };
    let me = me.ok_or(anyhow!("获取账号失败"))?;

    let key = format!("P2P:USER:AUTH:{}:{}", uuid, token);
    let result: RedisResult<String> = cmd("GET").arg(&key).query_async(&mut conn).await;
    let res = result?;
    info!("结果为 {} {}", uuid, res);
    match res == me {
        true => {
            let key = format!("{}{}", "USER_UDP_ADDRESS_", uuid);
            let result: RedisResult<String> = cmd("GET").arg(&key).query_async(&mut conn).await;
            Ok(CommonResponseRef::<String>::success_json(&result?)?)
        }
        false => Err(anyhow!("failed")),
    }
}

/// 添加用户验证的token
pub async fn add_p2p_token_service(
    uuid: String,
    token: String,
    me: Option<String>,
) -> Result<String, anyhow::Error> {
    let mut conn = get_redis_conn().await?;
    let me = me.ok_or(anyhow!("获取账号失败"))?;

    let key = format!("P2P:USER:AUTH:{}:{}", me, token);
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

    let mut basic_user = BasicUser::select_by_uuid(rb, &uuid).await?.ok_or(anyhow!("用户不存在"))?;
    let mut user_info = UserInfo::select_by_uuid(rb, &uuid).await?.ok_or(anyhow!("用户详情不存在"))?;

    update_dto.apply_to_basic_user(&mut basic_user);
    update_dto.apply_to_user_info(&mut user_info)?;

    let tx = rb.acquire_begin().await?;
    let result: Result<(), anyhow::Error> = async {
        BasicUser::update_by_map(&tx, &basic_user, value! { "uuid": &uuid }).await?;
        UserInfo::update_by_uuid(&tx, &user_info, &uuid).await?;
        tx.commit().await?;
        Ok(())
    }.await;

    if result.is_err() {
        let _ = tx.rollback().await;
        return Err(anyhow!("更新用户信息失败"));
    }

    Ok(CommonResponseNoDataRef::success_empty())
}

// pub async fn search_user_info() -> Result<String, anyhow::Error> {
//     let rb = RBATIS_DATABASE.read().await;
//     let rb = rb.as_ref().ok_or(anyhow!("获取连接失败"))?;
//     // 动态构建查询条件
//  }
