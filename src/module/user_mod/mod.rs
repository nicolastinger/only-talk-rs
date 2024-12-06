use actix_web::web;
use crate::module::user_mod::controller::user_controller::{user_service};

mod service;
mod model;
mod controller;

pub fn init_user_service(cfg: &mut web::ServiceConfig) {
    user_service(cfg)
}