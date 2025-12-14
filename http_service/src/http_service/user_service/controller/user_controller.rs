use crate::http_service::user_service::dto::basic_user_dto::SignInBasicUserDTO;
use crate::http_service::user_service::service::user_service::{
    add_new_basic_user_service, get_exit_user, get_user_info_by_account,
    get_user_info_by_uuid, get_user_uuid_by_account_service, user_sign_in
};
use actix_web::{post, web, HttpRequest, HttpResponse, Responder};
use log::{error, info};
use rbatis::RBatis;
use crate::http_service::user_service::dto::sign_up_basic_user_dto::SignUpBasicUserDTO;
use crate::utils::http_response::CommonResponse;
use crate::{get_uuid_from_header, respond_json_any, validate_and_respond};
use crate::utils::http_response::CommonResponseNoDataRef;
use crate::common::dto::base_dto::AuthAccount;

pub fn user_service(cfg: &mut web::ServiceConfig) {
    cfg.service(get_exit_user_flag)
        .service(sign_in)
        .service(sign_up)
        .service(me_api)
        .service(query_user_api)
        .service(get_user_uuid_by_account_api);
}


#[post("/get_exit_user_flag/is_exit")]
pub async fn get_exit_user_flag(state: web::Data<RBatis>, account: String) -> impl Responder {
    info!("获取到值 {}", account);
    let res = get_exit_user(state.get_ref(), &account).await;
    HttpResponse::Ok().body(res.to_string())
}


#[post("/sign_up")]
pub async fn sign_up(state: web::Data<RBatis>, basic_user: web::Json<SignUpBasicUserDTO>) -> impl Responder {
    let basic_user = validate_and_respond!(basic_user);
    let res = add_new_basic_user_service(state.get_ref(), basic_user).await;
    respond_json_any!(res)
}

#[post("/sign_in")]
pub async fn sign_in(
    state: web::Data<RBatis>,
    basic_user_dto: web::Json<SignInBasicUserDTO>,
) -> impl Responder {
    let basic_user_dto: SignInBasicUserDTO = validate_and_respond!(basic_user_dto);
    let res = user_sign_in(state.get_ref(), basic_user_dto).await;
    respond_json_any!(res)
}

#[post("/me")]
pub async fn me_api(state: web::Data<RBatis>, req: HttpRequest) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    let res = get_user_info_by_uuid(state.get_ref(), uuid).await;
    respond_json_any!(res)
}

#[post("/get_user_by_account/{account}")]
pub async fn query_user_api(
    state: web::Data<RBatis>,
    account: web::Path<String>,
) -> impl Responder {
    let account = account.into_inner();
    let res = get_user_info_by_account(state.get_ref(), Some(account)).await;
    respond_json_any!(res)
}

#[post("/get_uuid_by_account/{account}")]
pub async fn get_user_uuid_by_account_api(account: web::Path<String>) -> impl Responder {
    let account = account.into_inner();
    let res = get_user_uuid_by_account_service(account).await;
    respond_json_any!(res)
}

