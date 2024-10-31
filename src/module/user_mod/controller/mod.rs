use actix_web::web;
use crate::common::init_web::{redis_example, test_check};

mod user_controller;

pub fn user_service(cfg: &mut web::ServiceConfig) {
    cfg.service(redis_example)
        .service(test_check);
}