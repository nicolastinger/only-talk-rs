use actix_web::{HttpRequest, HttpResponse, Responder, post, web};
use tracing::error;

use crate::common::dto::base_dto::{AuthAccount, ReqList};
use crate::http_service::plaza_service::dto::plaza_dto::{
    PlazaCrushToggleDTO, PlazaListQuery, PlazaUpdateProfileDTO, PlazaUpdateTagsDTO,
};
use crate::http_service::plaza_service::service::plaza_service::{
    get_matched_list, get_my_liked_list, get_my_plaza_profile, get_plaza_list, get_plaza_user,
    switch_crush, update_my_plaza_profile, update_my_plaza_tags,
};
use crate::state::AppState;
use crate::utils::http_response::{CommonResponse, CommonResponseNoDataRef};
use crate::{get_uuid_from_header, respond_json_any};

pub fn plaza_service(cfg: &mut web::ServiceConfig) {
    cfg.service(my_profile_api)
        .service(update_profile_api)
        .service(update_tags_api)
        .service(list_api)
        .service(user_detail_api)
        .service(crush_switch_api)
        .service(like_list_api)
        .service(match_list_api);
}

fn succeed() -> HttpResponse {
    let body = CommonResponse::<bool>::success(true);
    HttpResponse::Ok()
        .body(serde_json::to_string(&body).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e)))
}

fn fail(t: &anyhow::Error) -> HttpResponse {
    error!("err_context {:?}", t);
    HttpResponse::BadRequest().body(CommonResponseNoDataRef::error_json(&t.to_string()))
}

#[post("/profile")]
pub async fn my_profile_api(req: HttpRequest, state: web::Data<AppState>) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    respond_json_any!(get_my_plaza_profile(state.db(), uuid).await)
}

#[post("/profile/update")]
pub async fn update_profile_api(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<PlazaUpdateProfileDTO>,
) -> impl Responder {
    let me = get_uuid_from_header!(req);
    let dto = body.into_inner();
    match update_my_plaza_profile(state.db(), me, dto).await {
        Ok(_) => succeed(),
        Err(t) => fail(&t),
    }
}

#[post("/tag/update")]
pub async fn update_tags_api(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<PlazaUpdateTagsDTO>,
) -> impl Responder {
    let me = get_uuid_from_header!(req);
    let dto = body.into_inner();
    match update_my_plaza_tags(state.db(), me, dto.tags).await {
        Ok(_) => succeed(),
        Err(t) => fail(&t),
    }
}

#[post("/list")]
pub async fn list_api(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<ReqList<PlazaListQuery>>,
) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    let body = body.into_inner();
    let page_num = body.page_num.unwrap_or(1);
    let page_size = body.page_size.unwrap_or(20);
    let query = body.data.unwrap_or(PlazaListQuery {
        gender: None,
        age_min: None,
        age_max: None,
        tag: None,
    });
    respond_json_any!(get_plaza_list(state.db(), uuid, page_num, page_size, query).await)
}

#[post("/user/{uuid}")]
pub async fn user_detail_api(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    let target = path.into_inner();
    respond_json_any!(get_plaza_user(state.db(), uuid, target).await)
}

#[post("/like/switch")]
pub async fn crush_switch_api(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<PlazaCrushToggleDTO>,
) -> impl Responder {
    let me = get_uuid_from_header!(req);
    let dto = body.into_inner();
    respond_json_any!(switch_crush(state.db(), me, dto).await)
}

#[post("/like/list")]
pub async fn like_list_api(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<ReqList<serde_json::Value>>,
) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    let body = body.into_inner();
    let page_num = body.page_num.unwrap_or(1);
    let page_size = body.page_size.unwrap_or(20);
    respond_json_any!(get_my_liked_list(state.db(), uuid, page_num, page_size).await)
}

#[post("/match/list")]
pub async fn match_list_api(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<ReqList<serde_json::Value>>,
) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    let body = body.into_inner();
    let page_num = body.page_num.unwrap_or(1);
    let page_size = body.page_size.unwrap_or(20);
    respond_json_any!(get_matched_list(state.db(), uuid, page_num, page_size).await)
}
