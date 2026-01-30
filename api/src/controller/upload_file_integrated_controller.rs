use actix_multipart::Multipart;
use actix_web::{HttpRequest, HttpResponse, Responder, post, web};
use http_service::common::dto::base_dto::AuthAccount;
use http_service::utils::http_response::CommonResponseNoDataRef;
use http_service::{get_uuid_from_header, respond_json_any};
use tracing::error;
use rbatis::RBatis;

use crate::service::upload_file_integrated_service::{download_pub_biz, upload_user_avatar};

pub fn upload_file_integrated_service(cfg: &mut web::ServiceConfig) {
    cfg.service(download_pub_biz_api).service(upload_user_avatar_api);
}

#[post("/upload/user_avatar")]
async fn upload_user_avatar_api(
    payload: Multipart,
    req: HttpRequest,
    state: web::Data<RBatis>,
) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    let res = upload_user_avatar(state.as_ref(), uuid, payload).await;
    respond_json_any!(res)
}

/**
 * 下载公开业务文件
 */
#[post("/download/pub_biz/{biz_id}")]
async fn download_pub_biz_api(
    state: web::Data<RBatis>,
    biz_id: web::Path<String>,
) -> impl Responder {
    let biz_id = biz_id.into_inner();
    let res = download_pub_biz(state.as_ref(), biz_id).await;
    match res {
        Ok(res) => res,
        Err(t) => {
            error!("err_context {:?}", t);
            error!("{}", t.backtrace());
            HttpResponse::BadRequest().body(CommonResponseNoDataRef::error_json(&t.to_string()))
        }
    }
}
