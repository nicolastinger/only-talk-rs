use actix_web::{HttpRequest, Responder, post, web};
use common::models::chat_entity::add_read_chat_record::AddReadChatRecordDTO;

use crate::common::dto::base_dto::AuthAccount;
use crate::common::dto::base_page_dto::BasePageDTO;
use crate::http_service::chat_service::service::text_msg_service::{
    add_user_chat_read, get_chat_by_limit, get_unread_chat_record,
};
use crate::state::AppState;
use crate::utils::http_response::CommonResponseNoDataRef;
use crate::{get_uuid_from_header, respond_json_any};

pub fn text_msg_service(cfg: &mut web::ServiceConfig) {
    cfg.service(get_chat_record_api)
        .service(add_read_chat_record_api)
        .service(get_unread_chat_record_api);
}

/// 获取特定好友的聊天记录
#[post("/get_chat_record/{uuid}")]
pub async fn get_chat_record_api(
    req: HttpRequest,
    state: web::Data<AppState>,
    friend_uuid: web::Path<String>,
    base_page: web::Json<BasePageDTO>,
) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    let uuid_clone = uuid.clone();
    respond_json_any!(
        get_chat_by_limit(state.db(), uuid_clone, friend_uuid.into_inner(), base_page.into_inner())
            .await
    )
}

// 获取当前用户未读消息
#[post("/get_unread_chat_record")]
pub async fn get_unread_chat_record_api(
    req: HttpRequest,
    state: web::Data<AppState>,
) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    respond_json_any!(get_unread_chat_record(state.db(), uuid).await)
}

// 用户已读消息
#[post("/add_read_chat_record")]
pub async fn add_read_chat_record_api(
    req: HttpRequest,
    state: web::Data<AppState>,
    chat_message_read: web::Json<Vec<AddReadChatRecordDTO>>,
) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    respond_json_any!(add_user_chat_read(state.redis(), uuid, chat_message_read.into_inner()).await)
}
