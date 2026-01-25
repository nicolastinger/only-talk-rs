use actix_web::web;

use crate::http_service::file_service::controller::file_controller::file_service;

pub mod controller;
pub mod service;
pub mod vo;

/// 初始化文件服务路由
pub fn init_file_service(cfg: &mut web::ServiceConfig) {
    file_service(cfg)
}
