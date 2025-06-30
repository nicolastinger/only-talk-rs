use crate::utils::dto::AuthAccount;
use actix_web::{post, web, HttpRequest, HttpResponse, Responder};
use nanoid::nanoid;
use rbatis::RBatis;
use uuid::Uuid;
use crate::{get_uuid_from_header, respond_json_any};
use crate::common::quic_network_service::models::text_msg::TextQuicMsg;
use crate::module::chat_msg_mod::service::text_msg_service::{add_user_chat_record, get_chat_by_limit};
use crate::module::common::dto::base_page_dto::BasePageDto;
use crate::utils::time::get_now_time_stamp_as_millis;

pub fn text_msg_service(cfg: &mut web::ServiceConfig) {
    cfg.service(get_chat_record_api);
}

/// 获取特定好友的聊天记录
#[post("/get_chat_record/{uuid}")]
pub async fn get_chat_record_api(req: HttpRequest, state: web::Data<RBatis>, friend_uuid: web::Path<String>,base_page: web::Json<BasePageDto>) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    let uuid_clone = uuid.clone();
    let str = "我是蔡徐坤".as_bytes().to_vec();
    let text_msg = TextQuicMsg {
        id: nanoid!(),
        text_type: 0,
        raw: str,
        recv_user: uuid.unwrap(),
        send_user: friend_uuid.to_string(),
        timestamp: get_now_time_stamp_as_millis().unwrap(),
    };
    add_user_chat_record(text_msg).await.unwrap();
    respond_json_any!(get_chat_by_limit(state.as_ref(),uuid_clone,friend_uuid.into_inner(), base_page.into_inner()).await)
}

// 获取当前用户聊天列表
// #[post("/get_chat_list")]
// pub async fn get_chat_list(req: HttpRequest, state: web::Data<RBatis>) -> impl Responder {
//     let uuid = get_uuid_from_header!(req);
// }