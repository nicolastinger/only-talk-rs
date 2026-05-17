use actix_web::{delete, get, post, put, web, HttpRequest, HttpResponse, Responder};
use rbatis::RBatis;
use tracing::info;

use crate::common::dto::base_dto::AuthAccount;
use crate::http_service::group_service::group_dto::{
    add_member_dto::AddMemberDTO,
    create_group_dto::CreateGroupDTO,
    group_message_history_dto::GroupMessageHistoryDTO,
    set_role_dto::SetRoleDTO,
    update_group_dto::UpdateGroupDTO,
};
use crate::http_service::group_service::group_service::{
    add_group_members_service, create_group_service, dissolve_group_service,
    get_group_info_service, get_group_members_service, get_group_message_history_service,
    get_my_groups_service, get_unread_group_messages_service, quit_group_service,
    remove_group_member_service, set_member_role_service, update_group_service,
};
use crate::utils::http_response::CommonResponse;
use crate::{get_uuid_from_header, validate_and_respond};

pub fn group_service(cfg: &mut web::ServiceConfig) {
    cfg.service(create_group)
        .service(get_group_info)
        .service(update_group)
        .service(dissolve_group)
        .service(get_my_groups)
        .service(get_group_members)
        .service(add_group_members)
        .service(remove_group_member)
        .service(quit_group)
        .service(set_member_role)
        .service(get_group_message_history)
        .service(get_unread_group_messages);
}

fn get_uuid(req: &HttpRequest) -> String {
    get_uuid_from_header!(req).unwrap_or_default()
}

fn respond_json<T: serde::Serialize>(res: anyhow::Result<T>) -> HttpResponse {
    match res {
        Ok(t) => HttpResponse::Ok()
            .body(serde_json::to_string(&CommonResponse::success(t)).unwrap()),
        Err(e) => HttpResponse::BadRequest()
            .body(serde_json::to_string(&CommonResponse::error(e.to_string(), "error".to_string())).unwrap()),
    }
}

fn respond_bool(res: anyhow::Result<bool>) -> HttpResponse {
    match res {
        Ok(t) => HttpResponse::Ok()
            .body(serde_json::to_string(&CommonResponse::success(t)).unwrap()),
        Err(e) => HttpResponse::BadRequest()
            .body(serde_json::to_string(&CommonResponse::error(e.to_string(), "error".to_string())).unwrap()),
    }
}

#[post("/create")]
pub async fn create_group(
    state: web::Data<RBatis>,
    req: HttpRequest,
    dto: web::Json<CreateGroupDTO>,
) -> impl Responder {
    let dto = validate_and_respond!(dto);
    let uuid = get_uuid(&req);
    info!("create_group uuid={:?}", uuid);
    let res = create_group_service(state.get_ref(), &uuid, dto).await;
    respond_json(res)
}

#[get("/info/{group_uuid}")]
pub async fn get_group_info(
    state: web::Data<RBatis>,
    group_uuid: web::Path<String>,
) -> impl Responder {
    let group_uuid = group_uuid.into_inner();
    let res = get_group_info_service(state.get_ref(), &group_uuid).await;
    respond_json(res)
}

#[put("/update")]
pub async fn update_group(
    state: web::Data<RBatis>,
    req: HttpRequest,
    dto: web::Json<UpdateGroupDTO>,
) -> impl Responder {
    let dto = validate_and_respond!(dto);
    let uuid = get_uuid(&req);
    let res = update_group_service(state.get_ref(), &uuid, dto).await;
    respond_bool(res)
}

#[delete("/dissolve/{group_uuid}")]
pub async fn dissolve_group(
    state: web::Data<RBatis>,
    req: HttpRequest,
    group_uuid: web::Path<String>,
) -> impl Responder {
    let group_uuid = group_uuid.into_inner();
    let uuid = get_uuid(&req);
    let res = dissolve_group_service(state.get_ref(), &uuid, &group_uuid).await;
    respond_bool(res)
}

#[get("/my/list")]
pub async fn get_my_groups(
    state: web::Data<RBatis>,
    req: HttpRequest,
) -> impl Responder {
    let uuid = get_uuid(&req);
    let res = get_my_groups_service(state.get_ref(), &uuid).await;
    respond_json(res)
}

#[get("/member/list/{group_uuid}")]
pub async fn get_group_members(
    state: web::Data<RBatis>,
    group_uuid: web::Path<String>,
) -> impl Responder {
    let group_uuid = group_uuid.into_inner();
    let res = get_group_members_service(state.get_ref(), &group_uuid).await;
    respond_json(res)
}

#[post("/member/add")]
pub async fn add_group_members(
    state: web::Data<RBatis>,
    req: HttpRequest,
    dto: web::Json<AddMemberDTO>,
) -> impl Responder {
    let dto = validate_and_respond!(dto);
    let uuid = get_uuid(&req);
    let res = add_group_members_service(state.get_ref(), &uuid, dto).await;
    respond_bool(res)
}

#[delete("/member/remove/{group_uuid}/{user_uuid}")]
pub async fn remove_group_member(
    state: web::Data<RBatis>,
    req: HttpRequest,
    path: web::Path<(String, String)>,
) -> impl Responder {
    let (group_uuid, target_uuid) = path.into_inner();
    let uuid = get_uuid(&req);
    let res = remove_group_member_service(state.get_ref(), &uuid, &group_uuid, &target_uuid).await;
    respond_bool(res)
}

#[post("/member/quit/{group_uuid}")]
pub async fn quit_group(
    state: web::Data<RBatis>,
    req: HttpRequest,
    group_uuid: web::Path<String>,
) -> impl Responder {
    let group_uuid = group_uuid.into_inner();
    let uuid = get_uuid(&req);
    let res = quit_group_service(state.get_ref(), &uuid, &group_uuid).await;
    respond_bool(res)
}

#[put("/member/set_role")]
pub async fn set_member_role(
    state: web::Data<RBatis>,
    req: HttpRequest,
    dto: web::Json<SetRoleDTO>,
) -> impl Responder {
    let dto = validate_and_respond!(dto);
    let uuid = get_uuid(&req);
    let res = set_member_role_service(state.get_ref(), &uuid, dto).await;
    respond_bool(res)
}

#[get("/message/history")]
pub async fn get_group_message_history(
    state: web::Data<RBatis>,
    req: HttpRequest,
    query: web::Query<GroupMessageHistoryDTO>,
) -> impl Responder {
    let uuid = get_uuid(&req);
    let dto = query.into_inner();
    let res = get_group_message_history_service(state.get_ref(), &uuid, dto).await;
    respond_json(res)
}

#[get("/message/unread")]
pub async fn get_unread_group_messages(
    state: web::Data<RBatis>,
    req: HttpRequest,
) -> impl Responder {
    let uuid = get_uuid(&req);
    let res = get_unread_group_messages_service(state.get_ref(), &uuid).await;
    respond_json(res)
}
