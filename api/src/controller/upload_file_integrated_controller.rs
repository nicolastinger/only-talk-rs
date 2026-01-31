use actix_multipart::Multipart;
use actix_web::{HttpRequest, Responder, post, web};
use http_service::common::dto::base_dto::AuthAccount;
use http_service::utils::http_response::CommonResponseNoDataRef;
use http_service::{get_uuid_from_header, respond_json_any};
use rbatis::RBatis;

use crate::service::upload_file_integrated_service::{upload_user_avatar, upload_user_chat_file};

pub fn upload_file_integrated_service(cfg: &mut web::ServiceConfig) {
    cfg.service(upload_user_avatar_api)
        .service(upload_user_chat_api);
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

#[post("/upload/user_chat/{friend_uuid}")]
async fn upload_user_chat_api(
    payload: Multipart,
    req: HttpRequest,
    state: web::Data<RBatis>,
    friend_uuid: web::Path<String>,
) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    let friend_uuid = friend_uuid.into_inner();
    let res = upload_user_chat_file(state.as_ref(), uuid, payload, friend_uuid).await;
    respond_json_any!(res)
}
