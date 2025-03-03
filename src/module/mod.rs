mod user_mod;
mod chat_msg_mod;

use actix_web::web;

use user_mod::init_user_service;
use user_mod::init_friend_service;

// 主路由配置函数
#[inline]
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/user")
            .configure(init_user_service),
    ).service(
        web::scope("/chat")
            .configure(chat_msg_mod::chat_service)
    ).service(
        web::scope("/friend")
            .configure(init_friend_service)
    );
}