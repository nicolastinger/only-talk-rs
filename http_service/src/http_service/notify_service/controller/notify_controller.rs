use crate::utils::http_response::CommonResponseNoDataRef;
use crate::common::dto::base_dto::AuthAccount;
use actix_web::{post, web, HttpRequest, Responder};
use rbatis::RBatis;
use crate::{get_uuid_from_header, respond_json_any};
use crate::http_service::notify_service::service::system_notification::get_user_unread_notification;

pub fn notify_service(cfg: &mut web::ServiceConfig) {
    cfg.service(get_user_unread_notification_api);
}

/// 获取特定好友的聊天记录
#[post("/get_user_unread_notification")]
pub async fn get_user_unread_notification_api(
    req: HttpRequest,
    state: web::Data<RBatis>
) -> impl Responder {
    let uuid = get_uuid_from_header!(req);

    respond_json_any!(
        get_user_unread_notification(state.get_ref(), uuid, Some(false)).await
    )
}