use crate::utils::dto::AuthAccount;
use actix_web::{post, web, HttpRequest, HttpResponse, Responder};
use rbatis::RBatis;
use crate::{get_uuid_from_header, respond_json_any};
use crate::module::chat_msg_mod::service::text_msg_service::get_chat_by_limit;


pub fn text_msg_service(cfg: &mut web::ServiceConfig) {
    cfg.service(get_chat_record);
}

#[post("/get_chat_record/{account}")]
pub async fn get_chat_record (req: HttpRequest, state: web::Data<RBatis>) -> impl Responder {
    let account = get_uuid_from_header!(req);
    respond_json_any!(get_chat_by_limit(state.as_ref(),account).await)
}