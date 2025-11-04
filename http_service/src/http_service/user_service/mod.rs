use crate::http_service::user_service::controller::friend_controller::friend_service;
use crate::http_service::user_service::controller::user_controller::user_service;
use actix_web::web;

mod controller;
pub mod dto;
pub mod service;
mod vo;

pub fn init_user_service(cfg: &mut web::ServiceConfig) {
    user_service(cfg)
}

pub fn init_friend_service(cfg: &mut web::ServiceConfig) {
    friend_service(cfg)
}
