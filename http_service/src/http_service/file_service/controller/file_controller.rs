use actix_multipart::Multipart;
use actix_web::{get, post, web, HttpMessage, HttpRequest, HttpResponse, Responder};
use deadpool_redis::Pool;
use crate::http_service;

pub fn file_service(cfg: &mut web::ServiceConfig) {
    cfg
        .service(upload_file_api);
}

#[get("/upload_file")]
pub async fn upload_file_api(state: web::Data<Pool>) -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/plain; charset=utf-8")
        .body("我真是蔡徐坤")
}