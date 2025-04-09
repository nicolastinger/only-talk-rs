use actix_web::{get, post, web, HttpResponse, Responder};
use deadpool_redis::Pool;
use deadpool_redis::redis::{cmd, RedisResult};
use log::info;
use rbatis::RBatis;
use crate::common::init_server::AppState;
use crate::module::user_mod::model::basic_user::BasicUser;
use crate::module::user_mod::service::local_user_service::{add_new_basic_user_service, get_exit_user, get_user_raw, test_sql, user_sign_in};
use crate::utils::http_response::CommonResponse;
use crate::utils::jwt_util::{decode_jwt, get_jwt};
use crate::{respond_json, respond_to_json, serde_json_to_string, validate_and_respond};
use crate::module::user_mod::dto::basic_user_dto::SignInBasicUserDTO;

pub fn user_service(cfg: &mut web::ServiceConfig) {
    cfg.service(user_test)
        .service(get_online_user_by_redis)
        .service(create_online_user)
        .service(post_test)
        .service(get_online_user_by_rbatis)
        .service(get_exit_user_flag)
        .service(get_token)
        .service(check_token)
        .service(sign_in)
        .service(sign_up)
        .service(sign_test)
        .service(post_online_user);
}

#[get("/user_test")]
pub async fn user_test() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/plain; charset=utf-8")
        .body("我真是蔡徐坤")
}

#[post("/user_test")]
pub async fn post_test() -> impl Responder {
    HttpResponse::Ok().body("蔡徐坤")
}


#[get("/online_user/redis/{path}")]
async fn get_online_user_by_redis(state: web::Data<Pool>, path: web::Path<String>) -> impl Responder {
    info!("请求进来了");
    let username = path.into_inner();
    let mut conn = state.get().await.expect("打开redis连接失败");

    // 查询 Redis 中的值
    let info: RedisResult<String> = cmd("GET").arg(&username).query_async(&mut conn).await;
    match info {
        Ok(info) => {
            // 将 Redis 中的值解析为 User 实体
            info!("redis获取到的值 {}", info);
            HttpResponse::Ok().body(info)
        },
        Err(_) => {
            // 如果没有找到用户信息，返回 404
            HttpResponse::NotFound().body("User not found")
        }
    }
}

#[post("/online_user/redis/add_user")]
async fn create_online_user(
    state: web::Data<AppState>,
    user: String
) -> impl Responder {
    info!("新增用户请求进来了");
    let key = user.to_uppercase() + ":LOGIN";

    let value = "ONLINE".to_string();
    let mut conn = state.redis_pool.get().await.expect("打开redis连接失败");

    let result: RedisResult<()> = cmd("SET").arg(key).arg(value).query_async(&mut conn).await;
    match result {
        Ok(_) => HttpResponse::Created().body(format!("User {} created", user)),
        Err(_) => HttpResponse::InternalServerError().body("Failed to create user"),
    }
}

#[post("/online_user/raw_sql_test")]
pub async fn post_online_user(
    state: web::Data<RBatis>
) -> impl Responder { 
    info!("新增用户请求进来了");
    get_user_raw(state).await;
    HttpResponse::Ok().body("something")
}

#[get("/online_user/rbatis_test")]
pub async fn get_online_user_by_rbatis(state: web::Data<RBatis>) -> impl Responder {
    info!("测试rbatis");
    let res = test_sql(state.get_ref()).await;
    HttpResponse::Ok().json(res)
}

#[post("/get_exit_user_flag/is_exit")]
pub async fn get_exit_user_flag(state: web::Data<RBatis>, account: String) -> impl Responder {
    info!("获取到值 {}" ,account);
    let res = get_exit_user(state.get_ref(), &account).await;
    HttpResponse::Ok().body(res.to_string())
}

#[post("/test_token/get")]
pub async fn get_token(account:String) -> impl Responder {
    let token = get_jwt(account).unwrap();
    let res = CommonResponse::success(token);
    HttpResponse::Ok().body(serde_json::to_string(&res).unwrap())
}

#[post("/test_token/check")]
pub async fn check_token(token:String) -> impl Responder {
    HttpResponse::Ok().body(decode_jwt(token.as_ref()).unwrap())
}

#[post("/sign_up")]
pub async fn sign_up(state: web::Data<RBatis>,basic_user:web::Json<BasicUser>) -> impl Responder {
    let basic_user = validate_and_respond!(basic_user);
    let res = add_new_basic_user_service(state.get_ref(),basic_user).await;
    println!("{:?}", res);
    respond_to_json!(res)
}

#[post("/sign_in")]
pub async fn sign_in(state: web::Data<RBatis>,basic_user_dto:web::Json<SignInBasicUserDTO>) -> impl Responder {
    let basic_user_dto: SignInBasicUserDTO = validate_and_respond!(basic_user_dto);

    let basic_user = BasicUser {
        uuid: None,
        id: None,
        username: None,
        account: Some(basic_user_dto.account.unwrap_or_default()),
        icon: None,
        info: None,
        password: Some(basic_user_dto.password.unwrap_or_default()),
    };
    let res =  user_sign_in(state.get_ref(),basic_user).await;
    respond_to_json!(res, "normal".to_string())
}

#[post("/sign_in_test")]
pub async fn sign_test(basic_user:web::Json<SignInBasicUserDTO>) -> impl Responder {
    let basic_user = validate_and_respond!(basic_user);

    respond_json!(serde_json_to_string!(&basic_user))
}



