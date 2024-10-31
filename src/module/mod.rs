mod user_mod;
mod chat_msg_mod;

use actix_web::web;
use crate::module::user_mod::user_service;

// 主路由配置函数
#[inline]
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/user")
            .configure(user_service),
    );
}