use actix_web::web;

use crate::http_service::moment_service::controller::moment_controller::moment_service;

mod controller;
pub mod dto;
pub mod service;
pub mod vo;

pub fn init_moment_service(cfg: &mut web::ServiceConfig) {
    moment_service(cfg)
}
