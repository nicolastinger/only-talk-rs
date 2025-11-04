pub mod chat_service;
pub mod init_server;
pub mod user_service;
pub mod integrated_service;
pub mod notify_service;

use actix_web::web;
use chat_service::init_chat_service;
use user_service::init_friend_service;
use user_service::init_user_service;
use integrated_service::init_user_integrated_service;

// 主路由配置函数
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/user").configure(init_user_service))
        .service(web::scope("/friend").configure(init_friend_service))
        .service(web::scope("/msg").configure(init_chat_service))
        .service(web::scope("/integrated").configure(init_user_integrated_service));
}
