pub mod controller;
pub mod dto;
pub mod service;

use actix_web::web;

use self::controller::session_controller;

pub fn init_session_service(cfg: &mut web::ServiceConfig) {
    session_controller(cfg);
}
