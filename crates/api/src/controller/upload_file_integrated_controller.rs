use actix_multipart::Multipart;
use actix_web::{HttpRequest, Responder, get, post, web};
use http_service::common::dto::base_dto::AuthAccount;
use http_service::http_service::file_service::model::file_type_config::get_file_type_config;
use http_service::state::AppState;
use http_service::utils::http_response::{CommonResponseNoDataRef, CommonResponseRef};
use http_service::{get_uuid_from_header, respond_json_any};

use crate::service::upload_file_integrated_service::{
    upload_group_avatar, upload_group_chat_file, upload_moment, upload_user_avatar,
    upload_user_chat_file,
};

pub fn upload_file_integrated_service(cfg: &mut web::ServiceConfig) {
    cfg.service(get_file_type_config_api)
        .service(upload_user_avatar_api)
        .service(upload_user_chat_api)
        .service(upload_group_chat_api)
        .service(upload_group_avatar_api)
        .service(upload_moment_api);
}

/// 上传文件类型白名单(扩展名 + MIME), 供客户端同步选择器过滤。
/// 公开匿名端点(auth_middleware IGNORED_PATHS 已放行)。
#[get("/file_type_config")]
async fn get_file_type_config_api() -> impl Responder {
    let res = get_file_type_config()
        .and_then(|config| CommonResponseRef::success_json(&config).map_err(anyhow::Error::from));
    respond_json_any!(res)
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
