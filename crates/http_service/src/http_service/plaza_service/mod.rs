use actix_web::web;

use crate::http_service::plaza_service::controller::plaza_controller::plaza_service;

mod controller;
pub mod dto;
pub mod service;
mod vo;

pub fn init_plaza_service(cfg: &mut web::ServiceConfig) {
    plaza_service(cfg)
}
