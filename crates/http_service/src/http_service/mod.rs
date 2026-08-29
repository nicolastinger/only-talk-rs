pub mod announcement_service;
pub mod chat_service;
pub mod file_service;
pub mod group_service;
pub mod moment_service;
pub mod notify_service;
pub mod plaza_service;
pub mod user_service;

use actix_web::web;
use announcement_service::init_announcement_service;
use chat_service::init_chat_service;
use group_service::init_group_service;
use moment_service::init_moment_service;
use plaza_service::init_plaza_service;
use user_service::{init_friend_service, init_user_service};

use crate::http_service::file_service::init_file_service;
use crate::http_service::notify_service::init_notify_service;

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/user").configure(init_user_service))
        .service(web::scope("/friend").configure(init_friend_service))
        .service(web::scope("/notify").configure(init_notify_service))
        .service(web::scope("/plaza").configure(init_plaza_service))
        .service(web::scope("/moment").configure(init_moment_service))
        .service(web::scope("/msg").configure(init_chat_service))
        .service(web::scope("/file").configure(init_file_service))
        .service(web::scope("/group").configure(init_group_service))
        .service(web::scope("/announcement").configure(init_announcement_service));
}
