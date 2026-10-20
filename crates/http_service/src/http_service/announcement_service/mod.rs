use actix_web::web;

use crate::http_service::announcement_service::controller::announcement_controller::announcement_service;

mod controller;
pub mod dto;
pub mod service;
pub mod vo;

pub fn init_announcement_service(cfg: &mut web::ServiceConfig) {
    announcement_service(cfg)
}
