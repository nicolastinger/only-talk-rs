use crate::http_service::user_service::dto::friend_dto::FriendDTO;
use crate::http_service::user_service::dto::friend_request_info_dto::FriendRequestInfoDTO;
use crate::http_service::user_service::service::friend_service::{
    add_friend, get_friend_by_id, get_friend_list, process_friend,
};
use actix_web::{get, post, web, HttpMessage, HttpRequest, HttpResponse, Responder};
use log::{error, info};
use rbatis::RBatis;
use crate::common::dto::base_dto::{AuthAccount, ReqList};
use crate::{get_uuid_from_header, respond_json_any, validate_and_respond};

pub fn friend_service(cfg: &mut web::ServiceConfig) {
    cfg.service(qry_friend_list)
        .service(add_friend_api)
        .service(get_friend_api)
        .service(qry_friend_test);
}

#[post("/friend_list")]
pub async fn qry_friend_list(
    req: HttpRequest,
    state: web::Data<RBatis>,
    friend: web::Json<ReqList<FriendDTO>>,
) -> impl Responder {
    let friend = validate_and_respond!(friend, "2");
    info!("{:?}", friend);
    let map = req.extensions();
    let account = map.get::<AuthAccount>().unwrap();
    info!("账号 {:?}", account);
    HttpResponse::Ok().body("not implemented")
}

#[get("/friend_test")]
pub async fn qry_friend_test(
    req: HttpRequest,
    state: web::Data<RBatis>,
    friend: web::Json<ReqList<FriendDTO>>,
) -> impl Responder {
    respond_json_any!(get_friend_by_id(state.as_ref()).await)
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

    respond_json_any!(add_friend(state.as_ref(), friend).await)
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
    respond_json_any!(process_friend(state.as_ref(), friend).await)
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
