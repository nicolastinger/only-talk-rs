use actix_web::{get, post, web, HttpResponse, Responder};
use deadpool_redis::redis::{cmd, RedisResult};
use log::info;
use crate::common::init_web::AppState;
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
async fn get_online_user_by_redis(state: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    info!("请求进来了");
    let username = path.into_inner();
    let mut conn = state.redis_pool.get().await.expect("打开redis连接失败");

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

