use crate::common::dto::base_dto::AuthAccount;
use std::fs;
use std::sync::Arc;
use actix_multipart::Multipart;
use actix_web::{HttpResponse, Responder, post, web, get, HttpRequest};
use tracing::error;
use rbatis::RBatis;
use s3_service::S3Client;
use common::config_str::USER_FILE_PUBLIC_DIR;
use crate::{get_uuid_from_header, respond_json_any};
use crate::http_service::file_service::service::biz_service::upload_original_file_by_biz_id;
use crate::http_service::file_service::service::file_service::{download_chat_file_by_id, download_link_chat_biz, download_link_pub_biz, download_pub_file_by_id};
use crate::utils::http_response::CommonResponseNoDataRef;

pub fn file_service(cfg: &mut web::ServiceConfig) {
    cfg.service(download_file_api)
        .service(download_pub_file_id_api)
        .service(download_chat_biz_api)
        .service(download_chat_file_api)
        .service(download_pub_biz_api)
        .service(upload_origin_file_by_biz_api)
        .service(download_private_file_api);
}

#[post("/download_file")]
pub async fn download_file_api() -> impl Responder {
    let test_icon_path = format!("{}{}", USER_FILE_PUBLIC_DIR, "kun.webp");
    let file_content = match fs::read(&test_icon_path) {
        Ok(content) => content,
        Err(e) => {
            error!("读取文件失败: {}", e);
            return HttpResponse::NotFound()
                .body(CommonResponseNoDataRef::error_json(&format!("文件不存在: {}", e)));
        }
    };
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
    s3_client: web::Data<Arc<S3Client>>,
    biz_id: web::Path<String>,
) -> impl Responder {
    let biz_id = biz_id.into_inner();
    let is_preview_bool = true;
    let s3_client = (*s3_client.into_inner()).clone();

    let res = download_link_pub_biz(state.as_ref(), Some(s3_client), biz_id, is_preview_bool).await;
    respond_json_any!(res)
}

/**
 * 通过业务id和文件id下载公开文件
 */
#[get("/download_pub_file/{biz_id}/{file_id}")]
pub async fn download_pub_file_id_api(
    state: web::Data<RBatis>,
    s3_client: web::Data<Arc<S3Client>>,
    params: web::Path<(String, String)>
) -> impl Responder {
    let (biz_id, file_id) = params.into_inner();
    let s3_client = (*s3_client.into_inner()).clone();
    let res = download_pub_file_by_id(state.as_ref(), Some(s3_client), biz_id, file_id).await;
    match res {
        Ok(res) => res,
        Err(t) => {
            error!("下载公开文件失败 {:?}, {}", t, t.backtrace());
            HttpResponse::BadRequest().body(CommonResponseNoDataRef::error_json(&t.to_string()))
        }
    }
}

/**
 * 获取聊天业务文件下载link
 */
#[post("/download_link/chat_biz/{biz_id}/{is_preview}")]
async fn download_chat_biz_api(
    req: HttpRequest,
    state: web::Data<RBatis>,
    s3_client: web::Data<Arc<S3Client>>,
    biz_id: web::Path<(String, String)>,
) -> impl Responder {
    let (biz_id,is_preview) = biz_id.into_inner();
    let uuid = get_uuid_from_header!(req);
    let is_preview_bool = match is_preview.as_str() {
        "1" => true,
        _ => false,
    };
    let s3_client = (*s3_client.into_inner()).clone();
    let res = download_link_chat_biz(state.as_ref(), Some(s3_client), uuid, biz_id, is_preview_bool).await;
    respond_json_any!(res)
}

/**
 * 通过业务id和文件id下载聊天文件
 */
#[get("/download_chat_file/{biz_id}/{file_id}")]
pub async fn download_chat_file_api(
    req: HttpRequest, 
    state: web::Data<RBatis>,
    s3_client: web::Data<Arc<S3Client>>,
    params: web::Path<(String, String)>
) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    let (biz_id, file_id) = params.into_inner();
    let s3_client = (*s3_client.into_inner()).clone();
    let res = download_chat_file_by_id(state.as_ref(), Some(s3_client), uuid, biz_id, file_id).await;

    match res {
        Ok(res) => res,
        Err(t) => {
            error!("下载公开文件失败 {:?}, {}", t, t.backtrace());
            HttpResponse::BadRequest().body(CommonResponseNoDataRef::error_json(&t.to_string()))
        }
    }
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

/**
 * 通过业务id上传原始文件
 */
#[post("/upload/origin_file")]
async fn upload_origin_file_by_biz_api(
    state: web::Data<RBatis>,
    s3_client: web::Data<Arc<S3Client>>,
    biz_id: web::Query<String>,
    biz_record_type: web::Query<String>,
    preview_id: web::Query<String>,
    req: HttpRequest,
    payload: Multipart
) -> impl Responder {
    let biz_id= biz_id.into_inner();
    let biz_record_type = biz_record_type.into_inner();
    let preview_id = preview_id.into_inner();
    let uuid = get_uuid_from_header!(req);
    let s3_client = (*s3_client.into_inner()).clone();
    let res = upload_original_file_by_biz_id(state.as_ref(), Some(s3_client), uuid, biz_id, biz_record_type, preview_id, payload).await;
    respond_json_any!(res)
}
