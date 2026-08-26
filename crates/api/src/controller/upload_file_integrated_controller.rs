use actix_multipart::Multipart;
use actix_web::{HttpRequest, Responder, post, web};
use http_service::common::dto::base_dto::AuthAccount;
use http_service::state::AppState;
use http_service::utils::http_response::CommonResponseNoDataRef;
use http_service::{get_uuid_from_header, respond_json_any};

use crate::service::upload_file_integrated_service::{
    upload_group_avatar, upload_group_chat_file, upload_moment, upload_user_avatar,
    upload_user_chat_file,
};

pub fn upload_file_integrated_service(cfg: &mut web::ServiceConfig) {
    cfg.service(upload_user_avatar_api)
        .service(upload_user_chat_api)
        .service(upload_group_chat_api)
        .service(upload_group_avatar_api)
        .service(upload_moment_api);
}

#[post("/upload/user_avatar")]
async fn upload_user_avatar_api(
    payload: Multipart,
    req: HttpRequest,
    state: web::Data<AppState>,
) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    let s3_client = state.s3.clone();
    let res = upload_user_avatar(state.db(), uuid, payload, s3_client).await;
    respond_json_any!(res)
}

#[post("/upload/user_chat/{friend_uuid}")]
async fn upload_user_chat_api(
    payload: Multipart,
    req: HttpRequest,
    state: web::Data<AppState>,
    friend_uuid: web::Path<String>,
) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    let friend_uuid = friend_uuid.into_inner();
    let s3_client = state.s3.clone();
    let res = upload_user_chat_file(state.db(), uuid, payload, friend_uuid, s3_client).await;
    respond_json_any!(res)
}

#[post("/upload/group_chat/{group_uuid}")]
async fn upload_group_chat_api(
    payload: Multipart,
    req: HttpRequest,
    state: web::Data<AppState>,
    group_uuid: web::Path<String>,
) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    let group_uuid = group_uuid.into_inner();
    let s3_client = state.s3.clone();
    let res = upload_group_chat_file(state.db(), uuid, payload, group_uuid, s3_client).await;
    respond_json_any!(res)
}

#[post("/upload/group_avatar/{group_uuid}")]
async fn upload_group_avatar_api(
    payload: Multipart,
    req: HttpRequest,
    state: web::Data<AppState>,
    group_uuid: web::Path<String>,
) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    let group_uuid = group_uuid.into_inner();
    let s3_client = state.s3.clone();
    let res = upload_group_avatar(state.db(), uuid, group_uuid, payload, s3_client).await;
    respond_json_any!(res)
}

#[post("/upload/moment")]
async fn upload_moment_api(
    payload: Multipart,
    req: HttpRequest,
    state: web::Data<AppState>,
) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    let s3_client = state.s3.clone();
    let res = upload_moment(state.db(), uuid, payload, s3_client).await;
    respond_json_any!(res)
}
