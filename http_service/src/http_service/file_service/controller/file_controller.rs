use std::fs;

use actix_web::{HttpResponse, Responder, post, web, get};
use tracing::error;
use rbatis::RBatis;
use entity::config_str::USER_FILE_PUBLIC_DIR;
use crate::http_service::file_service::service::file_service::{download_link_pub_biz, download_pub_file_by_id};
use crate::utils::http_response::CommonResponseNoDataRef;

pub fn file_service(cfg: &mut web::ServiceConfig) {
    cfg.service(download_file_api)
        .service(download_pub_file_id_api)
        .service(download_chat_file_api)
        .service(download_private_file_api);
}

#[post("/download_file")]
pub async fn download_file_api() -> impl Responder {
    let test_icon_path = format!("{}{}", USER_FILE_PUBLIC_DIR, "kun.webp");
    let file_content = fs::read(test_icon_path).expect("Failed to read file");
    // 返回文件内容作为二进制响应
    HttpResponse::Ok()
        .content_type("image/webp")
        .insert_header(("Content-Disposition", "attachment; filename=hello.jpg".to_string()))
        .body(file_content)
}

/**
 * 获取公开业务文件下载link
 */
#[post("/download_link/pub_biz/{biz_id}")]
async fn download_pub_biz_api(
    state: web::Data<RBatis>,
    biz_id: web::Path<String>,
    is_preview: web::Query<Option<String>>
) -> impl Responder {
    let biz_id = biz_id.into_inner();
    let is_preview = is_preview.into_inner();
    let mut is_preview_bool = true;
    if let Some(is_preview) = is_preview{
        if is_preview == "0" {
            is_preview_bool = false;
        }
    }
    let res = download_link_pub_biz(state.as_ref(), biz_id, is_preview_bool).await;
    match res {
        Ok(res) => res,
        Err(t) => {
            error!("err_context {:?}", t);
            error!("{}", t.backtrace());
            HttpResponse::BadRequest().body(CommonResponseNoDataRef::error_json(&t.to_string()))
        }
    }
}

/**
 * 通过业务id和文件id下载公开文件
 */
#[get("/download_pub_file/{biz_id}/{file_id}")]
pub async fn download_pub_file_id_api(state: web::Data<RBatis>, params: web::Path<(String, String)>) -> impl Responder {
    let (biz_id, file_id) = params.into_inner();
    let res = download_pub_file_by_id(state.as_ref(), biz_id, file_id).await;
    match res {
        Ok(res) => res,
        Err(t) => {
            error!("下载公开文件失败 {:?}, {}", t, t.backtrace());
            HttpResponse::BadRequest().body(CommonResponseNoDataRef::error_json(&t.to_string()))
        }
    }
}

/**
 * 通过业务id和文件id下载聊天文件
 */
#[get("/download_chat_file/{biz_id}/{file_id}")]
pub async fn download_chat_file_api(_params: web::Path<(String, String)>) -> impl Responder {
    // 返回文件内容作为二进制响应
    HttpResponse::Ok()
        .content_type("image/webp")
        .insert_header(("Content-Disposition", "attachment; filename=hello.jpg".to_string()))
        .body("")
}

/**
 * 通过业务id和文件id下载私密文件
 */
#[get("/download_private_file/{biz_id}/{file_id}")]
pub async fn download_private_file_api(_params: web::Path<(String, String)>) -> impl Responder {
    // 返回文件内容作为二进制响应
    HttpResponse::Ok()
        .content_type("image/webp")
        .insert_header(("Content-Disposition", "attachment; filename=hello.jpg".to_string()))
        .body("")
}
