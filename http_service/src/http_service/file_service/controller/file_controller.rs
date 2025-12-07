use std::fs;
use actix_multipart::Multipart;
use actix_web::{get, post, web, HttpMessage, HttpRequest, HttpResponse, Responder};
use deadpool_redis::Pool;
use entity::config_str::USER_FILE_PUBLIC_DIR;
use crate::http_service;
use crate::http_service::file_service::service::file_service::save_file_to_local;

pub fn file_service(cfg: &mut web::ServiceConfig) {
    cfg
        .service(download_file_api);
}

#[post("/download_file")]
pub async fn download_file_api(
    mut payload: Multipart,
    db: web::Data<rbatis::RBatis>,
) -> impl Responder {
    let test_icon_path =  format!("{}{}", USER_FILE_PUBLIC_DIR, "kun.webp");
    let file_content = fs::read(test_icon_path).expect("Failed to read file");
    // 返回文件内容作为二进制响应
    HttpResponse::Ok()
        .content_type("image/webp")
        .insert_header(("Content-Disposition", format!("attachment; filename=hello.jpg")))
        .body(file_content)
}