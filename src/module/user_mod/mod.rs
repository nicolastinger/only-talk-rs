use actix_web::web;
use crate::module::user_mod::controller::user_controller;
use crate::module::user_mod::controller::user_controller::{add_new_basic_user, check_token, create_online_user, get_exit_user_flag, get_online_user_by_rbatis, get_online_user_by_redis, get_token, post_online_user, user_service};

mod service;
mod model;
mod controller;

pub fn init_user_service(cfg: &mut web::ServiceConfig) {
    user_service(cfg)
}