use actix_web::web;

use crate::http_service::chat_service::controller::text_msg_controller::text_msg_service;

mod controller;
pub mod service;
mod vo;

pub fn init_chat_service(cfg: &mut web::ServiceConfig) {
    text_msg_service(cfg);
}
