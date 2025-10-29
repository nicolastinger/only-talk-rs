pub mod chat_service;
mod common;
pub(crate) mod init_server;
mod user_service;

use actix_web::web;

use crate::http_service::chat_service::init_chat_service;
use user_service::init_friend_service;
use user_service::init_user_service;

// 主路由配置函数
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/user").configure(init_user_service))
        .service(web::scope("/friend").configure(init_friend_service))
        .service(web::scope("/msg").configure(init_chat_service));
}
