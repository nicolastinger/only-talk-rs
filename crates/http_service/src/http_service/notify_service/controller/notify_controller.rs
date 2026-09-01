use actix_web::{HttpRequest, Responder, post, web};

use crate::common::dto::base_dto::AuthAccount;
use crate::http_service::notify_service::service::system_notification::{
    get_user_unread_notification, mark_notifications_read,
};
use crate::state::AppState;
use crate::utils::http_response::CommonResponseNoDataRef;
use crate::{get_uuid_from_header, respond_json_any};

pub fn notify_service(cfg: &mut web::ServiceConfig) {
    cfg.service(get_user_unread_notification_api).service(mark_read_api);
}

/// 获取特定好友的聊天记录
#[post("/get_user_unread_notification")]
pub async fn get_user_unread_notification_api(
    req: HttpRequest,
    state: web::Data<AppState>,
) -> impl Responder {
    let uuid = get_uuid_from_header!(req);

    respond_json_any!(get_user_unread_notification(state.db(), uuid, Some(false)).await)
}

/// 批量标记已读
#[post("/mark_read")]
pub async fn mark_read_api(
    req: HttpRequest,
    state: web::Data<AppState>,
    ids: web::Json<Vec<String>>,
) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    respond_json_any!(mark_notifications_read(state.db(), uuid, ids.into_inner()).await)
}
