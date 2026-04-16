use actix_web::{HttpRequest, Responder, post, web};
use http_service::common::dto::base_dto::AuthAccount;
use http_service::http_service::user_service::dto::friend_request_info_dto::FriendRequestInfoDTO;
use http_service::utils::http_response::CommonResponseNoDataRef;
use http_service::{get_uuid_from_header, respond_json_any};
use rbatis::RBatis;

use crate::service::user_integrated_service::{add_user_with_notify, process_friend_with_notify};

pub fn user_integrated_service(cfg: &mut web::ServiceConfig) {
    cfg.service(add_user_with_notify_api).service(process_friend_with_notify_api);
}

/// 添加用户并发送通知
#[post("/add_friend_with_notify")]
pub async fn add_user_with_notify_api(
    req: HttpRequest,
    state: web::Data<RBatis>,
    friend: web::Json<FriendRequestInfoDTO>,
) -> impl Responder {
    let me = get_uuid_from_header!(req);
    let mut friend = friend.into_inner();
    friend.request_user = me;

    respond_json_any!(add_user_with_notify(state.as_ref(), friend).await)
}

/// 处理好友请求并通知
#[post("/process_friend_with_notify")]
pub async fn process_friend_with_notify_api(
    req: HttpRequest,
    state: web::Data<RBatis>,
    friend: web::Json<FriendRequestInfoDTO>,
) -> impl Responder {
    let me = get_uuid_from_header!(req);
    let mut friend = friend.into_inner();
    friend.accept_user = me;
    let res = process_friend_with_notify(state.as_ref(), friend).await;
    respond_json_any!(res)
}
