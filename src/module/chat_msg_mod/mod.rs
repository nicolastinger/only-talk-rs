use actix_web::web;

mod controller;
mod entity;
mod service;
mod vo;

use crate::module::chat_msg_mod::controller::text_msg_controller::text_msg_service;

pub fn init_chat_service(cfg: &mut web::ServiceConfig) {
    text_msg_service(cfg);
}