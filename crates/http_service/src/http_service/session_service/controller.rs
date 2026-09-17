use actix_web::{HttpRequest, Responder, post, web};

use crate::common::dto::base_dto::AuthAccount;
use crate::http_service::session_service::dto::{SessionReadDTO, SessionSyncedDTO};
use crate::http_service::session_service::service::{report_session_read, report_session_synced};
use crate::state::AppState;
use crate::utils::http_response::CommonResponseNoDataRef;
use crate::{get_uuid_from_header, respond_json_any};

pub fn session_controller(cfg: &mut web::ServiceConfig) {
    cfg.service(session_read_api).service(session_synced_api);
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
