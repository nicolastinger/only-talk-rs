use actix_web::web;
use crate::module::user_mod::controller::user_controller;

mod service;
mod model;
mod dao;
mod controller;

pub fn user_service(cfg: &mut web::ServiceConfig) {
    cfg.service(user_controller::user_test)
       .service(user_controller::post_test);
}