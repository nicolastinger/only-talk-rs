use actix_web::web;

use crate::http_service::file_service::controller::file_controller::file_service;
use crate::http_service::file_service::controller::s3_controller::s3_service_routes;

pub mod controller;
pub mod service;
pub mod vo;
mod model;

/// 初始化文件服务路由
pub fn init_file_service(cfg: &mut web::ServiceConfig) {
    file_service(cfg);
    s3_service_routes(cfg);
}
