use actix_web::{HttpRequest, Responder, post, web};

use crate::common::dto::base_dto::AuthAccount;
use crate::http_service::notify_service::dto::{MarkReadDTO, NotificationListDTO};
use crate::http_service::notify_service::service::system_notification::{
    get_unread_summary, get_user_unread_notification, list_notifications, mark_notifications_read,
};
use crate::state::AppState;
use crate::utils::http_response::CommonResponseNoDataRef;
use crate::{get_uuid_from_header, respond_json_any};

pub fn notify_service(cfg: &mut web::ServiceConfig) {
    cfg.service(get_user_unread_notification_api)
        .service(notification_list_api)
        .service(unread_summary_api)
        .service(mark_read_api);
}

/// 获取用户未读通知(兼容旧客户端, 仅返回窗口内未读)
#[post("/get_user_unread_notification")]
pub async fn get_user_unread_notification_api(
    req: HttpRequest,
    state: web::Data<AppState>,
) -> impl Responder {
    let uuid = get_uuid_from_header!(req);

    respond_json_any!(get_user_unread_notification(state.db(), uuid, Some(false)).await)
}

/// 通知中心列表(keyset 分页 + 分类/已读过滤)
#[post("/list")]
pub async fn notification_list_api(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<NotificationListDTO>,
) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    respond_json_any!(list_notifications(state.db(), uuid, body.into_inner()).await)
}

/// 未读数汇总(角标 + 分类 tab)
#[post("/unread_summary")]
pub async fn unread_summary_api(req: HttpRequest, state: web::Data<AppState>) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    respond_json_any!(get_unread_summary(state.db(), uuid).await)
}

/// 批量标记已读(指定 id 或按分类全部已读)
#[post("/mark_read")]
pub async fn mark_read_api(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<MarkReadDTO>,
) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    respond_json_any!(mark_notifications_read(state.db(), uuid, body.into_inner()).await)
}
