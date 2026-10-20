use actix_web::{HttpRequest, Responder, post, web};

use crate::common::dto::base_dto::AuthAccount;
use crate::http_service::report_service::dto::report_dto::CreateReportDTO;
use crate::http_service::report_service::service::report_service::create_report;
use crate::state::AppState;
use crate::utils::http_response::CommonResponseNoDataRef;
use crate::{get_uuid_from_header, respond_json_any};

pub fn report_service(cfg: &mut web::ServiceConfig) {
    cfg.service(create_api);
}

#[post("/create")]
pub async fn create_api(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<CreateReportDTO>,
) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    let dto = body.into_inner();
    respond_json_any!(create_report(state.db(), uuid, dto).await)
}
