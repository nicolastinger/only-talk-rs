use actix_web::web;
use crate::module::user_mod::controller::friend_controller::friend_service;
use crate::module::user_mod::controller::user_controller::{user_service};

mod service;
mod model;
mod controller;
mod dto;
mod vo;

pub fn init_user_service(cfg: &mut web::ServiceConfig) {
    user_service(cfg)
}

pub fn init_friend_service(cfg: &mut web::ServiceConfig) {
    friend_service(cfg)
}