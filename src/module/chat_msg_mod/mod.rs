use actix_web::web;

mod controller;
mod msg_trait;
mod model;
use controller::text_msg_controller::get_chat_record;

pub fn chat_service(cfg: &mut web::ServiceConfig) {
    cfg.service(get_chat_record);
}