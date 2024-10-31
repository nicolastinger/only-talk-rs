use actix_web::web;
use crate::module::user_mod::controller::user_controller;
use crate::module::user_mod::controller::user_controller::{create_online_user, get_online_user_by_redis};

mod service;
mod model;
mod dao;
mod controller;

pub fn user_service(cfg: &mut web::ServiceConfig) {
    cfg.service(user_controller::user_test)
        .service(get_online_user_by_redis)
        .service(create_online_user)
       .service(user_controller::post_test);
}