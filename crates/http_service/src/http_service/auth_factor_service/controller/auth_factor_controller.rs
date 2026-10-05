use actix_web::{HttpRequest, Responder, post, web};

use crate::common::dto::base_dto::AuthAccount;
use crate::http_service::auth_factor_service::dto::auth_factor_dto::{
    AuthFactorIdDTO, CreateAuthFactorDTO, SendAuthFactorCodeDTO, UpdateAuthFactorDTO,
};
use crate::http_service::auth_factor_service::service::auth_factor_service::{
    create_auth_factor, delete_auth_factor, get_auth_factor_detail, list_auth_factors,
    send_auth_factor_code, update_auth_factor,
};
use crate::state::AppState;
use crate::utils::http_response::{CommonResponse, CommonResponseNoDataRef};
use crate::{get_uuid_from_header, respond_json_any, validate_and_respond};

pub fn auth_factor_service(cfg: &mut web::ServiceConfig) {
    cfg.service(send_code_api)
        .service(create_api)
        .service(list_api)
        .service(detail_api)
        .service(update_api)
        .service(delete_api);
}

/// 发送二次认证因素绑定验证码(当前仅 email)
#[post("/send_code")]
pub async fn send_code_api(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<SendAuthFactorCodeDTO>,
) -> impl Responder {
    let dto = validate_and_respond!(body);
    let uuid = get_uuid_from_header!(req);
    respond_json_any!(
        send_auth_factor_code(state.db(), state.redis(), &state.email, uuid, dto).await
    )
}

/// 新增二次认证因素(email 需验证码)
#[post("/create")]
pub async fn create_api(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<CreateAuthFactorDTO>,
) -> impl Responder {
    let dto = validate_and_respond!(body);
    let uuid = get_uuid_from_header!(req);
    respond_json_any!(create_auth_factor(state.db(), state.redis(), uuid, dto).await)
}

/// 当前用户的二次认证因素列表
#[post("/list")]
pub async fn list_api(req: HttpRequest, state: web::Data<AppState>) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    respond_json_any!(list_auth_factors(state.db(), uuid).await)
}

/// 二次认证因素详情
#[post("/detail")]
pub async fn detail_api(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<AuthFactorIdDTO>,
) -> impl Responder {
    let dto = validate_and_respond!(body);
    let uuid = get_uuid_from_header!(req);
    respond_json_any!(get_auth_factor_detail(state.db(), uuid, dto.id).await)
}

/// 更新二次认证因素
#[post("/update")]
pub async fn update_api(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<UpdateAuthFactorDTO>,
) -> impl Responder {
    let dto = validate_and_respond!(body);
    let uuid = get_uuid_from_header!(req);
    respond_json_any!(update_auth_factor(state.db(), uuid, dto).await)
}

/// 删除二次认证因素
#[post("/delete")]
pub async fn delete_api(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<AuthFactorIdDTO>,
) -> impl Responder {
    let dto = validate_and_respond!(body);
    let uuid = get_uuid_from_header!(req);
    respond_json_any!(delete_auth_factor(state.db(), uuid, dto.id).await)
}
