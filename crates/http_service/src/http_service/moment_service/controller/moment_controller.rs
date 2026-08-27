use actix_web::{HttpRequest, HttpResponse, Responder, post, web};
use tracing::error;

use crate::common::dto::base_dto::{AuthAccount, ReqList};
use crate::http_service::moment_service::dto::moment_dto::{
    AddCommentDTO, CommentListQuery, CreateMomentDTO, FollowToggleDTO, LikeListQuery,
    LikeToggleDTO, MomentListQuery,
};
use crate::http_service::moment_service::service::moment_service::{
    add_comment, create_moment, get_comments, get_like_list, get_moment_detail, get_moment_list,
    switch_follow, switch_like,
};
use crate::state::AppState;
use crate::utils::http_response::{CommonResponse, CommonResponseNoDataRef};
use crate::{get_uuid_from_header, respond_json_any};

pub fn moment_service(cfg: &mut web::ServiceConfig) {
    cfg.service(create_api)
        .service(list_api)
        .service(detail_api)
        .service(like_switch_api)
        .service(like_list_api)
        .service(follow_switch_api)
        .service(comment_api)
        .service(comment_list_api);
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

#[post("/create")]
pub async fn create_api(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<CreateMomentDTO>,
) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    let dto = body.into_inner();
    respond_json_any!(create_moment(state.db(), uuid, dto).await)
}

#[post("/list")]
pub async fn list_api(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<ReqList<MomentListQuery>>,
) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    let body = body.into_inner();
    let page_num = body.page_num.unwrap_or(1);
    let page_size = body.page_size.unwrap_or(20);
    let feed = body.data.as_ref().and_then(|d| d.feed.clone());
    let author_uuid = body.data.map(|d| d.author_uuid).unwrap_or_default();
    respond_json_any!(
        get_moment_list(state.db(), uuid, page_num, page_size, author_uuid, feed).await
    )
}

#[post("/detail/{moment_uuid}")]
pub async fn detail_api(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    let moment_uuid = path.into_inner();
    respond_json_any!(get_moment_detail(state.db(), uuid, moment_uuid).await)
}

#[post("/like/switch")]
pub async fn like_switch_api(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<LikeToggleDTO>,
) -> impl Responder {
    let me = get_uuid_from_header!(req);
    let dto = body.into_inner();
    match switch_like(state.db(), me, dto).await {
        Ok(_) => succeed(),
        Err(t) => fail(&t),
    }
}

#[post("/follow/switch")]
pub async fn follow_switch_api(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<FollowToggleDTO>,
) -> impl Responder {
    let me = get_uuid_from_header!(req);
    let dto = body.into_inner();
    match switch_follow(state.db(), me, dto).await {
        Ok(_) => succeed(),
        Err(t) => fail(&t),
    }
}

#[post("/comment")]
pub async fn comment_api(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<AddCommentDTO>,
) -> impl Responder {
    let me = get_uuid_from_header!(req);
    let dto = body.into_inner();
    respond_json_any!(add_comment(state.db(), me, dto).await)
}

#[post("/comment/list")]
pub async fn comment_list_api(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<ReqList<CommentListQuery>>,
) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    let body = body.into_inner();
    let moment_uuid = body.data.map(|d| d.moment_uuid).unwrap_or_default();
    let page_num = body.page_num.unwrap_or(1);
    let page_size = body.page_size.unwrap_or(20);
    respond_json_any!(get_comments(state.db(), uuid, moment_uuid, page_num, page_size).await)
}

#[post("/like/list")]
pub async fn like_list_api(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<ReqList<LikeListQuery>>,
) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    let body = body.into_inner();
    let moment_uuid = body.data.map(|d| d.moment_uuid).unwrap_or_default();
    let page_num = body.page_num.unwrap_or(1);
    let page_size = body.page_size.unwrap_or(20);
    respond_json_any!(get_like_list(state.db(), uuid, moment_uuid, page_num, page_size).await)
}
