use actix_web::{HttpRequest, Responder, post, web};

use crate::common::dto::base_dto::AuthAccount;
use crate::http_service::session_service::dto::{
    SessionControlDTO, SessionListDTO, SessionReadDTO, SessionSyncedDTO, SyncRequestDTO,
};
use crate::http_service::session_service::service::{
    delete_session, list_sessions, mute_session, pin_session, report_session_read,
    report_session_synced, sync_sessions,
};
use crate::state::AppState;
use crate::utils::http_response::{CommonResponseNoDataRef, CommonResponseRef};
use crate::{get_uuid_from_header, respond_json_any};

pub fn session_controller(cfg: &mut web::ServiceConfig) {
    cfg.service(session_read_api)
        .service(session_synced_api)
        .service(session_sync_api)
        .service(session_list_api)
        .service(session_pin_api)
        .service(session_mute_api)
        .service(session_delete_api);
}

#[post("/read")]
pub async fn session_read_api(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<SessionReadDTO>,
) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    respond_json_any!(report_session_read(state.db(), uuid, body.into_inner()).await)
}

#[post("/synced")]
pub async fn session_synced_api(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<SessionSyncedDTO>,
) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    respond_json_any!(report_session_synced(state.db(), uuid, body.into_inner()).await)
}

#[post("/sync")]
pub async fn session_sync_api(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<SyncRequestDTO>,
) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    let result = sync_sessions(state.db(), uuid, body.into_inner())
        .await
        .and_then(|vo| Ok(CommonResponseRef::success_json(&vo)?));
    respond_json_any!(result)
}

#[post("/list")]
pub async fn session_list_api(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<SessionListDTO>,
) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    let result = list_sessions(state.db(), uuid, body.into_inner())
        .await
        .and_then(|vo| Ok(CommonResponseRef::success_json(&vo)?));
    respond_json_any!(result)
}

#[post("/pin")]
pub async fn session_pin_api(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<SessionControlDTO>,
) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    let result = pin_session(state.db(), uuid, body.into_inner())
        .await
        .map(|_| CommonResponseNoDataRef::success_empty());
    respond_json_any!(result)
}

#[post("/mute")]
pub async fn session_mute_api(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<SessionControlDTO>,
) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    let result = mute_session(state.db(), uuid, body.into_inner())
        .await
        .map(|_| CommonResponseNoDataRef::success_empty());
    respond_json_any!(result)
}

#[post("/delete")]
pub async fn session_delete_api(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<SessionControlDTO>,
) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    let result = delete_session(state.db(), uuid, body.into_inner())
        .await
        .map(|_| CommonResponseNoDataRef::success_empty());
    respond_json_any!(result)
}
