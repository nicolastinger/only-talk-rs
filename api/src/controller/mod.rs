use actix_web::web;
use crate::controller::upload_file_integrated_controller::upload_file_integrated_service;
use crate::controller::user_integrated_controller::user_integrated_service;

pub mod user_integrated_controller;
mod upload_file_integrated_controller;

pub fn configure_api_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/integrated").configure(user_integrated_service))
        .service(web::scope("/file_integrated").configure(upload_file_integrated_service));
}

