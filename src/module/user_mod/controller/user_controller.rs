use actix_web::{get, post, web, HttpMessage, HttpRequest, HttpResponse, Responder};
use anyhow::Error;
use deadpool_redis::Pool;
use deadpool_redis::redis::{cmd, RedisResult};
use log::{info,error};
use rbatis::RBatis;
use crate::common::init_server::AppState;
use crate::module::user_mod::entity::basic_user::BasicUser;
use crate::module::user_mod::service::local_user_service::{add_new_basic_user_service, add_p2p_token_service, get_exit_user, get_user_info_by_account, get_user_info_by_uuid, get_user_raw, get_user_uuid_by_account_service, test_sql, user_sign_in, verify_p2p_token_service};
use crate::utils::http_response::CommonResponse;
use crate::utils::jwt_util::{decode_jwt, get_jwt};
use crate::{get_uuid_from_header, respond_json, respond_json_any, serde_json_to_string, validate_and_respond};
use crate::module::user_mod::dto::basic_user_dto::SignInBasicUserDTO;
use crate::utils::dto::{AuthAccount};

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
        .service(me_api)
        .service(query_user_api)
        .service(get_user_uuid_by_account_api)
        .service(verify_p2p_token_api)
        .service(add_p2p_token_api)
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
    respond_json_any!(res)
}

#[post("/sign_in")]
pub async fn sign_in(state: web::Data<RBatis>,basic_user_dto:web::Json<SignInBasicUserDTO>) -> impl Responder {
    let basic_user_dto: SignInBasicUserDTO = validate_and_respond!(basic_user_dto);
    let res =  user_sign_in(state.get_ref(),basic_user_dto).await;
    respond_json_any!(res)
}

#[post("/me")]
pub async fn me_api(state: web::Data<RBatis>,req: HttpRequest) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    let res = get_user_info_by_uuid(state.get_ref(),uuid).await;
    respond_json_any!(res)
}

#[post("/get_user_by_account/{account}")]
pub async fn query_user_api(state: web::Data<RBatis>,account: web::Path<(String)>) -> impl Responder {
    let account = account.into_inner();
    let res = get_user_info_by_account(state.get_ref(),Some(account)).await;
    respond_json_any!(res)
}

#[post("/sign_in_test")]
pub async fn sign_test(basic_user:web::Json<SignInBasicUserDTO>) -> impl Responder {
    let basic_user = validate_and_respond!(basic_user);

    respond_json!(serde_json_to_string!(&basic_user))
}

#[post("/get_uuid_by_account/{account}")]
pub async fn get_user_uuid_by_account_api(account: web::Path<(String)>)-> impl Responder {
    let account = account.into_inner();
    let res = get_user_uuid_by_account_service(account).await;
    respond_json_any!(res)
}

#[post("/verify_p2p_token/{uuid}/{token}")]
pub async fn verify_p2p_token_api(path: web::Path<(String, String)>, req: HttpRequest) -> impl Responder {
    let (uuid, token) = path.into_inner();
    let me = get_uuid_from_header!(req);
    respond_json_any!(verify_p2p_token_service(uuid,token,me).await)
}

#[post("/add_p2p_token/{uuid}/{token}")]
pub async fn add_p2p_token_api(path: web::Path<(String, String)>, req: HttpRequest) -> impl Responder {
    let (uuid, token) = path.into_inner();
    let me = get_uuid_from_header!(req);
    respond_json_any!(add_p2p_token_service(uuid,token,me).await)
}

// 查询用户信息
// #[post("/search/user_info")]
// pub async fn search_user_api(basic_user_dto:web::Json<SignInBasicUserDTO>) -> impl Responder {
//     let basic_user_dto = basic_user_dto.into_inner();
//     respond_json_any!(add_p2p_token_service(uuid,token,me).await)
// }


