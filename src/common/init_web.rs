use std::fs::File;
use std::sync::Arc;
use actix_web::{get, middleware, post, web, App, HttpResponse, HttpServer, Responder};
use fast_log::Config;
use fast_log::consts::LogSize;
use fast_log::plugin::file_split::RollingType;
use fast_log::plugin::packer::LogPacker;
use log::{error, info, LevelFilter};
use deadpool_redis::{Pool, PoolError,Config as dp_config, Runtime};
use deadpool_redis::redis::cmd;
use deadpool_redis::redis::ExpireOption::NONE;

struct AppState {
    redis_pool: Arc<Pool>,
}

fn init_redis() -> AppState {
    // 创建 Redis 连接池
    let mut config = dp_config::from_url("redis://:REDACTED_REDIS_PASSWORD@175.178.17.158:10279/");
    let pool = config.create_pool(Some(Runtime::Tokio1)).expect("Failed to create Redis pool");

    let m = AppState {
        redis_pool: Arc::new(pool),
    };
    m
}

//初始化异步web容器
pub async fn start_server() -> std::io::Result<()> {
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(init_redis()))
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
#[get("/user/{username}")]
async fn redis_example(state: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    info!("请求进来了");

    let username = path.into_inner();
    let mut res = state.redis_pool.get().await;

    let mut conn = match res {
        Ok(c) => c,
        Err(e) => {
            error!("打开redis连接失败 {}", e.to_string());
            return  HttpResponse::InternalServerError().finish();
        }
    };

    // 查询 Redis 中的值
    let info: Result<String, _> = cmd("GET").arg(&username).query_async(&mut conn).await;

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
        .service(test_check);
}