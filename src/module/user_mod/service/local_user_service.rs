use std::str::FromStr;
use crate::module::user_mod::dto::basic_user_dto::SignInBasicUserDTO;
use crate::module::user_mod::entity::basic_user::{get_raw_sql, BasicUser, BasicUserSalt};
use crate::module::user_mod::entity::user_info::UserInfo;
use crate::module::user_mod::vo::user_info::UserInfoVO;
use crate::utils::http_response::{CommonResponseNoDataRef, CommonResponseRef};
use crate::utils::jwt_util::{decode_jwt, get_jwt};
use crate::utils::rsa_util::{generate_random_string, hash_with_salt};
use crate::{RBATIS_DATABASE, REDIS_CLIENT};
use actix_web::{web, HttpResponse, Responder};
use anyhow::anyhow;
use deadpool_redis::redis::{cmd, AsyncCommands, RedisResult};
use log::{error, info};
use rbatis::RBatis;
use rbs::value;
use uuid::Uuid;
use crate::utils::redis_utils::get_redis_conn;
use crate::utils::time::get_now_time_stamp_as_millis;

pub async fn get_user_raw(rb: web::Data<RBatis>) {
    get_raw_sql(rb).await
}

pub async fn create_new_user(rb: web::Data<RBatis>) {
    get_raw_sql(rb).await
}

pub async fn test_sql(rb: &RBatis) -> Vec<BasicUser> {
    let basic_user_all = BasicUser::select_all(rb).await.unwrap();
    let basic_user_icon = BasicUser::select_by_map(rb, value!{ "icon": "33333" })
        .await
        .unwrap();
    let basic_user_all_id = BasicUser::select_all_by_id(rb, "33333", "4444444")
        .await
        .unwrap();
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
    mut basic_user: BasicUser,
) -> Result<String, anyhow::Error> {
    basic_user.uuid = Some(Uuid::now_v7().to_string().parse()?);
    let random_str = generate_random_string(16);
    let password = hash_with_salt(basic_user.password.as_ref().unwrap(), &random_str);
    basic_user.password = Option::from(password);

    let account_ref: &str = basic_user
        .account
        .as_ref()
        .map(|s| s.as_str())
        .unwrap_or("");
    match get_exit_user(rb, &account_ref).await {
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
                    age: None,
                    birthday: None,
                    note: None,
                    create_at: Some(now),
                    update_at: Some(now),
                    last_login_at: None,
                    last_login_equipment: None,
                    last_login_ipv4: None,
                    last_login_ipv6: None,
                    phone: None,
                    email: None,
                    address: None,
                    status: None,
                };

                BasicUserSalt::insert(rb, &basic_user_salt).await?;
                BasicUser::insert(rb, &basic_user).await?;
                UserInfo::insert(rb, &user_info).await?;
                
                tx.commit().await?;
                Ok(())
            }.await;
            
            // 如果事务中有错误，回滚事务
            if result.is_err() {
                let _ = tx.rollback().await;
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
    // 解构 basic_user 以获取 account 和 password 的引用
    let basic_user = BasicUser {
        uuid: None,
        username: None,
        account: basic_user_dto.account,
        icon: None,
        info: None,
        password: basic_user_dto.password,
    };

    let BasicUser {
        account, password, ..
    } = basic_user;

    // 验证 account 和 password 是否存在
    let account_str = account.as_ref().ok_or(anyhow!("账号为空".to_string()))?;
    let password_str = password.as_ref().ok_or(anyhow!("密码为空".to_string()))?;

    // 查询用户
    let basic_user = BasicUser::select_by_account(rb, account_str).await?.ok_or(anyhow!("用户不存在"))?;

    // 查询盐值信息
    let salt_vec = BasicUserSalt::select_by_map(rb, value!{ "uuid": &basic_user.uuid }).await?;

    // 检查盐值是否存在
    let salt = salt_vec.first().ok_or(anyhow!("密码不存在!".to_string()))?;

    // 哈希输入密码
    let hashed_password = hash_with_salt(
        password_str,
        salt.sign_up_salt
            .as_ref()
            .ok_or(anyhow!("加密盐查询失败".to_string()))?,
    );

    let exit_password = basic_user.password.as_ref().ok_or(anyhow!("账号为空"))?;

    // 比较哈希后的密码
    if exit_password == &hashed_password {
        // 生成 JWT
        Ok(CommonResponseRef::<String>::success_json(&get_jwt(
            basic_user.uuid.ok_or(anyhow!("账号为空"))?.to_string(),
        )?)?)
    } else {
        Err(anyhow!("用户或密码不正确!"))
    }
}

pub async fn get_user_info_by_account(
    rbatis: &RBatis,
    account: Option<String>,
) -> Result<String, anyhow::Error> {
    let account = account.ok_or(anyhow!("账号为空"))?;

    let basic_user = BasicUser::select_by_account(rbatis, &account)
        .await?
        .ok_or(anyhow!("查询为空"))?;
    let uuid = basic_user.uuid.as_ref().unwrap();
    let user_info = UserInfo::select_by_uuid(rbatis, uuid)
        .await?
        .ok_or(anyhow!("查询为空"))?;
    let user_info_vo = UserInfoVO::from((user_info, basic_user));
    Ok(CommonResponseRef::<UserInfoVO>::success_json(
        &user_info_vo,
    )?)
}

pub async fn get_user_info_by_uuid(
    rbatis: &RBatis,
    uuid: Option<String>,
) -> Result<String, anyhow::Error> {
    let uuid = uuid.ok_or(anyhow!("账号为空"))?;
    let uuid  = rbatis::rbdc::Uuid::from_str(uuid.as_str())?;

    let basic_user = BasicUser::select_by_uuid(rbatis, &uuid)
        .await?
        .ok_or(anyhow!("查询为空"))?;
    let uuid = basic_user.uuid.as_ref().unwrap();
    let user_info = UserInfo::select_by_uuid(rbatis, uuid)
        .await?
        .ok_or(anyhow!("查询为空"))?;
    let user_info_vo = UserInfoVO::from((user_info, basic_user));
    Ok(CommonResponseRef::<UserInfoVO>::success_json(
        &user_info_vo,
    )?)
}

/// 获取用户的uuid
pub async fn get_user_uuid_by_account_service(
    account: String,
) -> Result<String, anyhow::Error> {
    let result = get_user_uuid_by_account(account).await?;
    Ok(CommonResponseRef::<String>::success_json(
        &result.to_string(),
    )?)
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
            let basic_user = BasicUser::select_by_account(rb, &account)
                .await?
                .ok_or(anyhow!("账号不存在"))?;
            basic_user.uuid.ok_or(anyhow!("账号id为空"))?
        }
    };
    // 设置24小时的缓存
    cmd("SET")
        .arg(&key)
        .arg(uuid.to_string())
        .arg("EX")
        .arg(86400)
        .query_async(&mut conn)
        .await?;
    Ok(uuid.to_string().parse()?)
}

/// 验证用户传递的token
pub async fn verify_p2p_token_service(uuid: String, token: String, me: Option<String>) -> Result<String, anyhow::Error> {
    let mut conn = {
        let redis_client = REDIS_CLIENT.read().await;
        let redis_conn = redis_client.as_ref().ok_or(anyhow!("redis客户端错误"))?;
        let mut conn = redis_conn.get().await?;
        conn
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
        },
        false => Err(anyhow!("failed"))
    }
}

/// 添加用户验证的token
pub async fn add_p2p_token_service(uuid: String, token: String, me: Option<String>) -> Result<String, anyhow::Error> {
    let mut conn = get_redis_conn().await?;
    let me = me.ok_or(anyhow!("获取账号失败"))?;

    let key = format!("P2P:USER:AUTH:{}:{}", me, token);
    cmd("SET")
        .arg(&key)
        .arg(uuid.to_string())
        .arg("EX")
        .arg(600)
        .query_async(&mut conn)
        .await?;
    Ok(CommonResponseNoDataRef::success_empty())
}

// pub async fn search_user_info() -> Result<String, anyhow::Error> {
//     let rb = RBATIS_DATABASE.read().await;
//     let rb = rb.as_ref().ok_or(anyhow!("获取连接失败"))?;
//     // 动态构建查询条件
//  }
