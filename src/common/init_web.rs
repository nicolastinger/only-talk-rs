use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
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
use rbatis::RBatis;
use rbdc_mysql::MysqlDriver;
use rustls::{Certificate, PrivateKey, ServerConfig};
use rustls::server::NoClientAuth;
use rustls_pemfile::{certs, ec_private_keys, rsa_private_keys, pkcs8_private_keys};
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

fn init_db() -> Arc<RBatis> {
    let rb = RBatis::new();
    rb.init(MysqlDriver{}, "mysql://rust_dev:REDACTED_DB_PASSWORD_REMOTE#@175.178.17.158:10222/test")
        .expect("Failed to initialize database connection");
    Arc::new(rb)
}


//初始化异步web容器
pub async fn start_server() -> std::io::Result<()> {
    // 加载证书
    let cert_file = &mut BufReader::new(File::open("config/TLS/onlytalk.cn.pem")?);
    let key_file = &mut BufReader::new(File::open("config/TLS/onlytalk.cn.key")?);

    // 读取证书链
    let cert_chain = match certs(cert_file) {
        Ok(certs) => {
            eprintln!("读取到 {} 个证书", certs.len());
            certs.into_iter().map(Certificate).collect()
        },
        Err(e) => {
            eprintln!("无法读取证书文件: {}", e);
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "无法读取证书文件"));
        }
    };

    // 读取私钥
    let mut key_content = String::new();
    key_file.read_to_string(&mut key_content)?;
    eprintln!("私钥文件内容:\n{}", key_content);

    key_file.seek(SeekFrom::Start(0)).expect("无法重置文件读取位置");

    // 从.key文件加载私钥
    let mut key_file = BufReader::new(File::open("config/TLS/onlytalk.cn.key").expect("打开key文件失败"));
    let mut keys = rsa_private_keys(&mut key_file).or_else(|_| ec_private_keys(&mut key_file))
        .map_err(|_| "无法解析私钥文件").unwrap();

    if keys.is_empty() {
        eprintln!("私钥文件中没有找到有效的私钥");
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "私钥文件中没有找到有效的私钥"));
    }

    let key = PrivateKey(keys.remove(0));

    // 配置 TLS
    let config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(cert_chain, key)
        .map_err(|e| {
            eprintln!("无法设置证书和私钥: {}", e);
            std::io::Error::new(std::io::ErrorKind::Other, "无法设置证书和私钥")
        })?;

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
        .bind_rustls_021("127.0.0.1:8443",config)? // 绑定到 HTTPS 端口
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
    let mut conn = state.redis_pool.get().await.expect("打开redis连接失败");

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