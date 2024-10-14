use std::fs::File;
use actix_web::{get, middleware, post, web, App, HttpResponse, HttpServer, Responder};
use fast_log::Config;
use fast_log::consts::LogSize;
use fast_log::plugin::file_split::RollingType;
use fast_log::plugin::packer::LogPacker;
use log::{info, LevelFilter};
use crate::{RB, REDIS_CONNECTION};
use redis::{Commands, Client as RedisClient, RedisResult, RedisError};
use sqlx::mysql::MySqlPool;
use sqlx::Error;

//初始化异步web容器
pub async fn start_server() -> std::io::Result<()> {
    HttpServer::new(move || {
        App::new()
            // 设置中间件，让actix-web打印日志
            .wrap(middleware::Logger::default())
            .route("/", web::get().to(home))

            .configure(|cfg: &mut web::ServiceConfig| {
                cfg.service(web::scope("/etc").configure(config_service));
            })
        // 这里可以继续添加其他路由
    })
        .bind("127.0.0.1:8090")?
        .run()
        .await
}


// 假设有一个结构体来表示用户信息，这里简化处理，直接返回一个静态字符串
pub async fn home() -> String {
    info!("Home");
    "hello,world!".to_string()
}

// 使用 Redis
#[get("/redis")]
async fn redis_example() -> String {
    info!("请求进来了");

    // 获取异步连接
    let mut con = REDIS_CONNECTION.lock().unwrap();
    let result = "test_doc".to_string();
    // 获取一个键值对
    let res: RedisResult<String> = con.get(result);
    let real_key = res.unwrap_or_else(|e| "没有数据".to_string());
    real_key
}

#[get("/test_mysql")]
async fn test_post(user_id: String) -> impl Responder {
    match sqlx::query!("SELECT * FROM users WHERE id = ?", id)
        .fetch_one(RB.lock().as_ref())
        .await
    {
        Ok(user) => HttpResponse::Ok().json(user),
        Err(e) => HttpResponse::InternalServerError().body(format!("Error: {}", e)),
    }
    info!("什么才是什么 {}", user_id);
    HttpResponse::Ok().body("测试post")
}

#[get("/check")]
async fn test_check() -> impl Responder {
    let f = File::open("hello.txt");

    let f = match f {
        Ok(file) => file,
        Err(error) => {
            panic!("Problem opening the file: {:?}", error)
        }
    };
    HttpResponse::Ok().body("")
}

// 导出处理函数以便在外部使用
pub fn config_service(cfg: &mut web::ServiceConfig) {
    cfg.service(redis_example)
        .service(test_check)
        .service(test_post);
}