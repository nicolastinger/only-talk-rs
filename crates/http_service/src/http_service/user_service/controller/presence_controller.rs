use actix_web::{HttpRequest, HttpResponse, Responder, post, web};

use crate::common::dto::base_dto::AuthAccount;
use crate::http_service::user_service::dto::presence_dto::{PresenceBatchDTO, SetPresenceDTO};
use crate::http_service::user_service::service::presence_service::{
    get_friend_presence, get_my_presence, set_presence,
};
use crate::state::AppState;
use crate::utils::http_response::CommonResponseNoDataRef;
use crate::{get_uuid_from_header, respond_json_any};

pub fn presence_service(cfg: &mut web::ServiceConfig) {
    cfg.service(set_presence_api).service(my_presence_api).service(friend_presence_api);
}

/// 设置/清除自己的手动状态
#[post("/presence/set")]
pub async fn set_presence_api(
    req: HttpRequest,
    state: web::Data<AppState>,
    dto: web::Json<SetPresenceDTO>,
) -> impl Responder {
    let uuid = match get_uuid_from_header!(req) {
        Some(uuid) => uuid,
        None => return HttpResponse::Unauthorized().body("Unauthorized"),
    };
    let dto = dto.into_inner();
    respond_json_any!(set_presence(state.redis(), &uuid, &dto.mode, dto.text.as_deref()).await)
}

/// 读取自己的手动状态
#[post("/presence/mine")]
pub async fn my_presence_api(req: HttpRequest, state: web::Data<AppState>) -> impl Responder {
    let uuid = match get_uuid_from_header!(req) {
        Some(uuid) => uuid,
        None => return HttpResponse::Unauthorized().body("Unauthorized"),
    };
    respond_json_any!(get_my_presence(state.redis(), &uuid).await)
}

/// 批量读取好友的手动状态(非好友 uuid 会被忽略)
#[post("/presence/batch")]
pub async fn friend_presence_api(
    req: HttpRequest,
    state: web::Data<AppState>,
    dto: web::Json<PresenceBatchDTO>,
) -> impl Responder {
    let uuid = match get_uuid_from_header!(req) {
        Some(uuid) => uuid,
        None => return HttpResponse::Unauthorized().body("Unauthorized"),
    };
    let dto = dto.into_inner();
    respond_json_any!(get_friend_presence(state.redis(), state.db(), &uuid, &dto.uuids).await)
}
