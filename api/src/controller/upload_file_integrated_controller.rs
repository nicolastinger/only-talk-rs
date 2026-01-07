use http_service::utils::http_response::CommonResponseNoDataRef;
use http_service::common::dto::base_dto::AuthAccount;
use actix_web::{post, web, HttpRequest, Responder};
use actix_multipart::Multipart;
use rbatis::RBatis;
use http_service::{get_uuid_from_header, respond_json_any};
use crate::service::upload_file_integrated_service::{upload_file_local_, upload_user_avatar};

pub fn upload_file_integrated_service(cfg: &mut web::ServiceConfig) {
    cfg.service(upload_image_api)
               .service(upload_user_avatar_api);
}

#[post("/upload")]
async fn upload_image_api(payload: Multipart) -> impl Responder {
    let res = upload_file_local_(payload).await;
    res
}

#[post("/upload/user_avatar")]
async fn upload_user_avatar_api(payload: Multipart, req: HttpRequest, state: web::Data<RBatis>,) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    let res = upload_user_avatar(state.as_ref(), uuid, payload).await;
    respond_json_any!(res)
}