use actix_web::web;
use crate::controller::user_integrated_controller::user_integrated_service;

pub mod user_integrated_controller;

pub fn configure_api_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/integrated").configure(user_integrated_service));
}

