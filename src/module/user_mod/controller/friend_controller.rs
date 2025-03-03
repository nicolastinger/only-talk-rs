use actix_web::{get, post, web, HttpResponse, Responder};
use rbatis::RBatis;
use crate::module::user_mod::model::basic_user::BasicUser;
use crate::module::user_mod::service::local_user_service::add_new_basic_user_service;
use crate::{respond_to_json, validate_and_respond};
use crate::module::user_mod::dto::friend_dto::Friend;
use crate::utils::dto::ReqList;
use crate::utils::jwt_util::decode_jwt;

pub fn friend_service(cfg: &mut web::ServiceConfig) {
    cfg.service(qry_friend_list);
}

#[post("/friend_list")]
pub async fn qry_friend_list(state: web::Data<RBatis>,friend: web::Json<ReqList<Friend>>) -> impl Responder {
    let friend = validate_and_respond!(friend,"2");
    println!("{:?}", friend);
    HttpResponse::Ok().body("not implemented")
}