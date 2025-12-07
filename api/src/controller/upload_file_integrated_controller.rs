use actix_web::{post, web, Responder};
use actix_multipart::Multipart;
use crate::service::upload_file_integrated_service::upload_file_local;

pub fn upload_file_integrated_service(cfg: &mut web::ServiceConfig) {
    cfg.service(upload_image_api);
}

#[post("/upload")]
async fn upload_image_api(payload: Multipart) -> impl Responder {
    let res = upload_file_local(payload).await;
    res
}