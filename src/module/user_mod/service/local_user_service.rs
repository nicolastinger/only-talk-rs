use crate::module::user_mod::model::basic_user::{get_raw_sql, BasicUser, BasicUserSalt};
use crate::utils::rsa_util::{generate_random_string, hash_with_salt};
use actix_web::{web, HttpResponse};
use log::{error, info};
use rbatis::rbdc::db::ExecResult;
use rbatis::RBatis;
use rbs::Error;
use std::future::Future;
use uuid::Uuid;
use crate::utils::jwt_util::get_jwt;

pub async fn get_user_raw(rb: web::Data<RBatis>) {
    get_raw_sql(rb).await
}

pub async fn create_new_user(rb: web::Data<RBatis>) {
    get_raw_sql(rb).await
}

pub async fn test_sql(rb: &RBatis) -> Vec<BasicUser> {
    let basic_user_all = BasicUser::select_all(rb).await.unwrap();
    let basic_user_icon = BasicUser::select_by_column(rb, "icon", "33333")
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

pub async fn get_exit_user(rb: &RBatis, account: String) -> bool {
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
) -> Result<String, String> {
    // 生成一个版本 4 的 UUID
    let uuid_v4 = Uuid::new_v4();
    basic_user.uuid = Option::from(uuid_v4.to_string());
    let random_str = generate_random_string(16);
    let password = hash_with_salt(basic_user.password.as_ref().unwrap(), &random_str);
    basic_user.password = Option::from(password);
    match get_exit_user(rb, basic_user.account.clone().unwrap()).await {
        true => Err("该账号已存在!".parse().unwrap()),
        false => {
            BasicUserSalt::insert(
                rb,
                &BasicUserSalt {
                    uuid: uuid_v4.to_string(),
                    sign_up_salt: random_str.to_string(),
                },
            )
            .await
            .unwrap();
            match BasicUser::insert(rb, &basic_user).await {
                Ok(_) => Ok("新增账号成功!".to_string()),
                Err(_) => Err("新增账号失败!".parse().unwrap()),
            }
        }
    }
}

pub async fn user_sign_in(rb: &RBatis, basic_user: BasicUser) -> Result<String, String> {
    // 解构 basic_user 以获取 account 和 password 的引用
    let BasicUser { account, password, .. } = basic_user;

    // 验证 account 和 password 是否存在
    let account_str = account.as_ref().ok_or("账号为空")?;
    let password_str = password.as_ref().ok_or("密码为空")?;

    // 查询用户
    let basic_user_vec = BasicUser::select_by_column(rb, "account", account_str)
        .await
        .map_err(|_| "用户查询失败!".to_string())?;

    // 检查用户是否存在
    let basic_user_exit = basic_user_vec.first().ok_or("用户不存在!".to_string())?;

    // 查询盐值信息
    let salt_vec = BasicUserSalt::select_by_column(rb, "uuid", &basic_user_exit.uuid)
        .await
        .map_err(|_| "用户密码查询失败!".to_string())?;

    // 检查盐值是否存在
    let salt = salt_vec.first().ok_or("密码不存在!".to_string())?;

    // 哈希输入密码
    let hashed_password = hash_with_salt(password_str, salt.sign_up_salt.as_ref());

    // 比较哈希后的密码
    if basic_user_exit.password.as_deref() == Some(&hashed_password) {
        // 生成 JWT
        get_jwt(account_str.clone())
            .map_err(|_| "生成token失败!".to_string())
    } else {
        Err("用户或密码不正确!".to_string())
    }
}
