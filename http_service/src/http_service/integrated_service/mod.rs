use actix_web::web::ServiceConfig;
use controller::user_integrated_controller::user_integrated_service;

mod controller;
pub mod service;

/// 本模块为整合各个模块的融合api中心，模块间通过此模块进行调用

pub fn init_user_integrated_service(cfg: &mut ServiceConfig) {
    user_integrated_service(cfg)
}

