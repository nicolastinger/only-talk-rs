use actix_web::{HttpRequest, Responder, post, web};

use crate::common::dto::base_dto::{AuthAccount, ReqList};
use crate::http_service::announcement_service::dto::announcement_dto::AnnouncementListQuery;
use crate::http_service::announcement_service::service::announcement_service::{
    get_announcement_detail, get_announcement_list, get_announcement_read_users,
    mark_announcement_read,
};
use crate::state::AppState;
use crate::utils::http_response::CommonResponseNoDataRef;
use crate::{get_uuid_from_header, respond_json_any};

pub fn announcement_service(cfg: &mut web::ServiceConfig) {
    cfg.service(list_api).service(detail_api).service(read_api).service(read_list_api);
}

#[post("/list")]
pub async fn list_api(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<ReqList<AnnouncementListQuery>>,
) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    let body = body.into_inner();
    let page_num = body.page_num.unwrap_or(1);
    let page_size = body.page_size.unwrap_or(20);
    respond_json_any!(get_announcement_list(state.db(), uuid, page_num, page_size).await)
}

#[post("/detail/{announcement_uuid}")]
pub async fn detail_api(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    let announcement_uuid = path.into_inner();
    respond_json_any!(get_announcement_detail(state.db(), uuid, announcement_uuid).await)
}

#[post("/read/{announcement_uuid}")]
pub async fn read_api(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    let announcement_uuid = path.into_inner();
    respond_json_any!(mark_announcement_read(state.db(), uuid, announcement_uuid).await)
}

#[post("/read/list/{announcement_uuid}")]
pub async fn read_list_api(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<ReqList<AnnouncementListQuery>>,
) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    let announcement_uuid = path.into_inner();
    let body = body.into_inner();
    let page_num = body.page_num.unwrap_or(1);
    let page_size = body.page_size.unwrap_or(20);
    respond_json_any!(
        get_announcement_read_users(state.db(), uuid, announcement_uuid, page_num, page_size).await
    )
}
