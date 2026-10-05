use actix_web::web;

use crate::http_service::auth_factor_service::controller::auth_factor_controller::auth_factor_service;

mod controller;
pub mod dto;
pub mod service;
pub mod vo;

pub fn init_auth_factor_service(cfg: &mut web::ServiceConfig) {
    auth_factor_service(cfg)
}
