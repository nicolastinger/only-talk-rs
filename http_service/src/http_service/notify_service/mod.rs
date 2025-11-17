use actix_web::web;
use crate::http_service::notify_service::controller::notify_controller::notify_service;

pub mod service;
mod controller;

pub fn init_notify_service(cfg: &mut web::ServiceConfig) {
    notify_service(cfg);
}