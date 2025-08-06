use crate::utils::dto::AuthAccount;
use actix_web::{get, post, web, HttpMessage, HttpRequest, HttpResponse, Responder};
use log::{info,error};
use rbatis::RBatis;
use crate::{get_uuid_from_header, respond_json_any, validate_and_respond};
use crate::module::user_mod::dto::friend_dto::{FriendDTO, FriendLinkDTO};
use crate::module::user_mod::service::friend_service::{add_friend, get_friend_by_id, get_friend_list};
use crate::utils::dto::{ReqList};

pub fn friend_service(cfg: &mut web::ServiceConfig) {
    cfg.service(qry_friend_list)
        .service(add_friend_api)
        .service(get_friend_api)
        .service(qry_friend_test);
}


#[post("/friend_list")]
pub async fn qry_friend_list(req: HttpRequest, state: web::Data<RBatis>,friend: web::Json<ReqList<FriendDTO>>) -> impl Responder {
    let friend = validate_and_respond!(friend,"2");
    info!("{:?}", friend);
    let map = req.extensions();
    let account = map.get::<AuthAccount>().unwrap();
    info!("账号 {:?}", account);
    HttpResponse::Ok().body("not implemented")
}

#[get("/friend_test")]
pub async fn qry_friend_test(req: HttpRequest, state: web::Data<RBatis>,friend: web::Json<ReqList<FriendDTO>>) -> impl Responder {
    respond_json_any!(get_friend_by_id(state.as_ref()).await)
}

#[post("/add_friend")]
pub async fn add_friend_api(req: HttpRequest, state: web::Data<RBatis>, friend: web::Json<FriendLinkDTO>) -> impl Responder {
    let friend = validate_and_respond!(friend);
    let account = get_uuid_from_header!(req);
    
    respond_json_any!(add_friend(state.as_ref(), account, friend.account).await)
}

#[post("/get_friend/{last_uuid}")]
pub async fn get_friend_api(req: HttpRequest, state: web::Data<RBatis>, last_uuid: web::Path<String>) -> impl Responder {
    let uuid = get_uuid_from_header!(req);

    respond_json_any!(get_friend_list(state.as_ref(), uuid, last_uuid.into_inner()).await)
}

