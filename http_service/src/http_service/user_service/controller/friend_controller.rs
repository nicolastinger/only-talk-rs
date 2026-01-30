use actix_web::{HttpMessage, HttpRequest, HttpResponse, Responder, post, web};
use tracing::{error, info};
use rbatis::RBatis;

use crate::common::dto::base_dto::{AuthAccount, ReqList};
use crate::http_service::user_service::dto::friend_dto::FriendDTO;
use crate::http_service::user_service::dto::friend_request_info_dto::FriendRequestInfoDTO;
use crate::http_service::user_service::service::friend_service::{
    add_friend, get_accept_friend_request_list, get_friend_list, get_friend_request_list,
    process_friend,
};
use crate::utils::http_response::CommonResponseNoDataRef;
use crate::{get_uuid_from_header, respond_json_any, validate_and_respond};

pub fn friend_service(cfg: &mut web::ServiceConfig) {
    cfg.service(qry_friend_list)
        .service(add_friend_api)
        .service(get_friend_api)
        .service(get_accept_friend_request_list_api)
        .service(get_friend_request_list_api);
}

#[post("/friend_list")]
pub async fn qry_friend_list(
    req: HttpRequest,
    friend: web::Json<ReqList<FriendDTO>>,
) -> impl Responder {
    let friend = validate_and_respond!(friend, "2");
    info!("{:?}", friend);
    let map = req.extensions();
    let account = map.get::<AuthAccount>().expect("no auth account");
    info!("friend {:?}", friend);
    info!("账号 {:?}", account);
    HttpResponse::Ok().body("not implemented")
}

#[post("/add_friend")]
pub async fn add_friend_api(
    req: HttpRequest,
    state: web::Data<RBatis>,
    friend: web::Json<FriendRequestInfoDTO>,
) -> impl Responder {
    let me = get_uuid_from_header!(req);
    let mut friend = friend.into_inner();
    friend.request_user = me;
    let res = add_friend(state.as_ref(), friend).await;
    if let Err(t) = res {
        error!("err_context {:?}", t);
        error!("{}", t.backtrace());
        return HttpResponse::BadRequest()
            .body(CommonResponseNoDataRef::error_json(&t.to_string()));
    }
    let res = CommonResponseNoDataRef::success_empty();
    HttpResponse::Ok().body(res)
}

#[post("/process_friend")]
pub async fn process_friend_api(
    req: HttpRequest,
    state: web::Data<RBatis>,
    friend: web::Json<FriendRequestInfoDTO>,
) -> impl Responder {
    let me = get_uuid_from_header!(req);
    let mut friend = friend.into_inner();
    friend.accept_user = me;
    let res = process_friend(state.as_ref(), friend).await;
    if let Err(t) = res {
        error!("err_context {:?}", t);
        error!("{}", t.backtrace());
        return HttpResponse::BadRequest()
            .body(CommonResponseNoDataRef::error_json(&t.to_string()));
    }
    let res = CommonResponseNoDataRef::success_empty();
    HttpResponse::Ok().body(res)
}

#[post("/get_friend/{last_uuid}/{version}")]
pub async fn get_friend_api(
    req: HttpRequest,
    state: web::Data<RBatis>,
    path: web::Path<(String, String)>,
) -> impl Responder {
    let (last_uuid, version) = path.into_inner();
    let uuid = get_uuid_from_header!(req);

    respond_json_any!(get_friend_list(state.as_ref(), uuid, last_uuid, version).await)
}

#[post("/get_accept_friend_request_list")]
pub async fn get_accept_friend_request_list_api(
    req: HttpRequest,
    state: web::Data<RBatis>,
    friend: web::Json<FriendRequestInfoDTO>,
) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    let accept_status = friend.into_inner().accept_status;

    respond_json_any!(get_accept_friend_request_list(state.as_ref(), uuid, accept_status).await)
}

#[post("/get_friend_request_list")]
pub async fn get_friend_request_list_api(
    req: HttpRequest,
    state: web::Data<RBatis>,
    friend: web::Json<FriendRequestInfoDTO>,
) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    let accept_status = friend.into_inner().accept_status;

    respond_json_any!(get_friend_request_list(state.as_ref(), uuid, accept_status).await)
}
