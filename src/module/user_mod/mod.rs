use actix_web::web;
use crate::module::user_mod::controller::user_controller;
use crate::module::user_mod::controller::user_controller::{add_new_basic_user, check_token, create_online_user, get_exit_user_flag, get_online_user_by_rbatis, get_online_user_by_redis, get_token, post_online_user};

mod service;
mod model;
mod controller;

pub fn user_service(cfg: &mut web::ServiceConfig) {
    cfg.service(user_controller::user_test)
        .service(get_online_user_by_redis)
        .service(create_online_user)
        .service(user_controller::post_test)
        .service(get_online_user_by_rbatis)
        .service(get_exit_user_flag)
        .service(add_new_basic_user)
        .service(get_token)
        .service(check_token)
        .service(post_online_user);
}