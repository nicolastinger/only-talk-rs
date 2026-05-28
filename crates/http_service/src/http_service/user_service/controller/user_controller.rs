use actix_web::{HttpRequest, HttpResponse, Responder, get, post, web};
use tracing::info;
use rbatis::RBatis;

use crate::common::dto::base_dto::AuthAccount;
use crate::http_service::user_service::dto::basic_user_dto::SignInBasicUserDTO;
use crate::http_service::user_service::dto::refresh_token_dto::RefreshTokenDTO;
use crate::http_service::user_service::dto::sign_up_basic_user_dto::SignUpBasicUserDTO;
use crate::http_service::user_service::dto::update_user_dto::UpdateUserDTO;
use crate::http_service::user_service::service::user_service::{
    add_new_basic_user_service, get_exit_user, get_user_info_by_account, get_user_info_by_uuid,
    get_user_uuid_by_account_service, refresh_access_token, update_user_info_service, user_sign_in,
};
use crate::utils::http_response::{CommonResponse, CommonResponseNoDataRef};
use crate::{get_uuid_from_header, respond_json_any, validate_and_respond};

pub fn user_service(cfg: &mut web::ServiceConfig) {
    cfg.service(get_exit_user_flag)
        .service(sign_in)
        .service(sign_up)
        .service(refresh_token)
        .service(me_api)
        .service(query_user_api)
        .service(get_user_by_uuid_api)
        .service(get_user_uuid_by_account_api)
        .service(update_user_info_api);
}

#[post("/get_exit_user_flag/is_exit")]
pub async fn get_exit_user_flag(state: web::Data<RBatis>, account: String) -> impl Responder {
    info!("获取到值 {}", account);
    let res = get_exit_user(state.get_ref(), &account).await;
    HttpResponse::Ok().body(res.to_string())
}

#[post("/sign_up")]
pub async fn sign_up(
    state: web::Data<RBatis>,
    basic_user: web::Json<SignUpBasicUserDTO>,
) -> impl Responder {
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

#[post("/refresh_token")]
pub async fn refresh_token(
    dto: web::Json<RefreshTokenDTO>,
) -> impl Responder {
    let dto: RefreshTokenDTO = validate_and_respond!(dto);
    let res = refresh_access_token(dto).await;
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

#[get("/get_user_by_uuid/{uuid}")]
pub async fn get_user_by_uuid_api(
    state: web::Data<RBatis>,
    uuid: web::Path<String>,
) -> impl Responder {
    let uuid = uuid.into_inner();
    info!("获取用户信息 by uuid: {}", uuid);
    let res = get_user_info_by_uuid(state.get_ref(), Some(uuid)).await;
    respond_json_any!(res)
}

#[post("/get_uuid_by_account/{account}")]
pub async fn get_user_uuid_by_account_api(account: web::Path<String>) -> impl Responder {
    let account = account.into_inner();
    let res = get_user_uuid_by_account_service(account).await;
    respond_json_any!(res)
}

#[post("/update")]
pub async fn update_user_info_api(
    state: web::Data<RBatis>,
    req: HttpRequest,
    update_dto: web::Json<UpdateUserDTO>,
) -> impl Responder {
    let update_dto = validate_and_respond!(update_dto);
    let uuid = get_uuid_from_header!(req);
    let res = update_user_info_service(state.get_ref(), uuid, update_dto).await;
    respond_json_any!(res)
}
