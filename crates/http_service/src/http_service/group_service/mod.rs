pub mod group_controller;
pub mod group_service;
pub mod group_dto;
pub mod group_vo;

use actix_web::web;

use self::group_controller::group_service;

pub fn init_group_service(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/chat").configure(group_service));
}
