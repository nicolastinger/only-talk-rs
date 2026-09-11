use actix_web::web;

use crate::http_service::report_service::controller::report_controller::report_service;

mod controller;
pub mod dto;
pub mod service;

pub fn init_report_service(cfg: &mut web::ServiceConfig) {
    report_service(cfg)
}
