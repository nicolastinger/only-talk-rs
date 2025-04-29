use crate::module::user_mod::model::basic_user::{get_raw_sql, BasicUser, BasicUserSalt, UserInfo};
use crate::utils::rsa_util::{generate_random_string, hash_with_salt};
use actix_web::{web};
use anyhow::anyhow;
use log::{error, info};
use rbatis::RBatis;
use uuid::Uuid;
use crate::module::user_mod::dto::basic_user_dto::SignInBasicUserDTO;
use crate::utils::http_response::CommonResponseRef;
use crate::utils::jwt_util::get_jwt;
use crate::module::user_mod::vo::user::UserInfoVO;

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

    let account_ref: &str = basic_user.account.as_ref().map(|s| s.as_str()).unwrap_or("");
    match get_exit_user(rb, &account_ref).await {
        true => Err(anyhow!("该账号已存在!".to_string())),
        false => {
            BasicUserSalt::insert(
                rb,
                &BasicUserSalt {
                    uuid: basic_user.uuid.clone(),
                    sign_up_salt: Option::from(random_str.to_string()),
                },
            )
            .await?;
            match BasicUser::insert(rb, &basic_user).await {
                Ok(_) => Ok(CommonResponseRef::<String>::success_json(&"新增账号成功".to_string())?),
                Err(_) => Err(anyhow!("新增账号失败!".to_string())),
            }
        }
    }
}

/// 用户登录
pub async fn user_sign_in(rb: &RBatis, basic_user_dto: SignInBasicUserDTO) -> Result<String, anyhow::Error> {
    // 解构 basic_user 以获取 account 和 password 的引用
    let basic_user = BasicUser{
        uuid: None,
        username: None,
        account: basic_user_dto.account,
        icon: None,
        info: None,
        password: basic_user_dto.password,
    };

    let BasicUser { account, password, .. } = basic_user;

    // 验证 account 和 password 是否存在
    let account_str = account.as_ref().ok_or(anyhow!("账号为空".to_string()))?;
    let password_str = password.as_ref().ok_or(anyhow!("密码为空".to_string()))?;

    // 查询用户
    let basic_user_vec = BasicUser::select_by_column(rb, "account", account_str)
        .await?;

    // 检查用户是否存在
    let basic_user_exit = basic_user_vec.first().ok_or(anyhow!("用户不存在!".to_string()))?;

    // 查询盐值信息
    let salt_vec = BasicUserSalt::select_by_column(rb, "uuid", &basic_user_exit.uuid)
        .await?;

    // 检查盐值是否存在
    let salt = salt_vec.first().ok_or(anyhow!("密码不存在!".to_string()))?;

    // 哈希输入密码
    let hashed_password = hash_with_salt(password_str, salt.sign_up_salt.as_ref().ok_or(anyhow!("加密盐查询失败".to_string()))?);

    // 比较哈希后的密码
    if basic_user_exit.password.as_deref() == Some(&hashed_password) {
        // 生成 JWT
        Ok(CommonResponseRef::<String>::success_json(&get_jwt(account_str.clone())?)?)
    } else {
        Err(anyhow!("用户或密码不正确!"))
    }
}

pub async fn me(rbatis: &RBatis, account: Option<String>)-> Result<String, anyhow::Error> {
    let user_info = UserInfo::select_by_account(rbatis, account.ok_or(anyhow!("账号为空"))?).await?;
    let user_info_vo = UserInfoVO::from(user_info.ok_or(anyhow!("查询为空"))?);
    Ok(CommonResponseRef::<UserInfoVO>::success_json(&user_info_vo)?)
}
