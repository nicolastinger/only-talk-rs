use actix_web::web;

mod controller;
mod entity;
mod service;

use controller::text_msg_controller::get_chat_record;

pub fn chat_service(cfg: &mut web::ServiceConfig) {
    cfg.service(get_chat_record);
}