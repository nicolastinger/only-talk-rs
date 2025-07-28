use crate::utils::dto::AuthAccount;
use actix_web::{post, web, HttpRequest, HttpResponse, Responder};
use rbatis::RBatis;
use crate::{get_uuid_from_header, respond_json_any};
use crate::module::chat_msg_mod::entity::chat_message_read::{ChatMessageRecordRead};
use crate::module::chat_msg_mod::service::text_msg_service::{add_user_chat_read, get_chat_by_limit, get_chat_list_link, get_unread_chat_record};
use crate::module::common::dto::base_page_dto::BasePageDto;

pub fn text_msg_service(cfg: &mut web::ServiceConfig) {
    cfg.service(get_chat_record_api)
        .service(add_read_chat_record_api)
        .service(get_unread_chat_record_api)
        .service(get_chat_list);
}

/// 获取特定好友的聊天记录
#[post("/get_chat_record/{uuid}")]
pub async fn get_chat_record_api(req: HttpRequest, state: web::Data<RBatis>, friend_uuid: web::Path<String>,base_page: web::Json<BasePageDto>) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    let uuid_clone = uuid.clone();
    respond_json_any!(get_chat_by_limit(state.as_ref(),uuid_clone,friend_uuid.into_inner(), base_page.into_inner()).await)
}

// 获取当前用户聊天列表
#[post("/get_chat_list")]
pub async fn get_chat_list(req: HttpRequest, state: web::Data<RBatis>) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    respond_json_any!(get_chat_list_link(state.as_ref(), uuid).await)
}

// 获取当前用户未读消息
#[post("/get_unread_chat_record")]
pub async fn get_unread_chat_record_api(req: HttpRequest, state: web::Data<RBatis>) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    respond_json_any!(get_unread_chat_record(state.as_ref(), uuid).await)
}

// 用户已读消息
#[post("/add_read_chat_record")]
pub async fn add_read_chat_record_api(req: HttpRequest, chat_message_read: web::Json<Vec<ChatMessageRecordRead>>) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    respond_json_any!(add_user_chat_read(uuid, chat_message_read.into_inner()).await)
}