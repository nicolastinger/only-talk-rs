use actix_web::{HttpRequest, HttpResponse, Responder, get, post, web};
use http_service::common::dto::base_dto::AuthAccount;
use http_service::http_service::user_service::dto::friend_request_info_dto::FriendRequestInfoDTO;
use http_service::state::AppState;
use http_service::utils::http_response::CommonResponseNoDataRef;
use http_service::{get_uuid_from_header, respond_json_any};

use crate::service::user_integrated_service::{
    add_user_with_notify, get_nat_udp_ports, get_quic_server_for_user, process_friend_with_notify,
};

pub fn user_integrated_service(cfg: &mut web::ServiceConfig) {
    cfg.service(add_user_with_notify_api)
        .service(process_friend_with_notify_api)
        .service(get_quic_server_for_user_api)
        .service(get_nat_udp_ports_api);
}

/// 添加好友并发送通知
#[post("/add_friend_with_notify")]
pub async fn add_user_with_notify_api(
    req: HttpRequest,
    state: web::Data<AppState>,
    friend: web::Json<FriendRequestInfoDTO>,
) -> impl Responder {
    let me = get_uuid_from_header!(req);
    let mut friend = friend.into_inner();
    friend.request_user = me;

    respond_json_any!(add_user_with_notify(state.db(), friend).await)
}

/// 处理好友请求并发送通知
#[post("/process_friend_with_notify")]
pub async fn process_friend_with_notify_api(
    req: HttpRequest,
    state: web::Data<AppState>,
    friend: web::Json<FriendRequestInfoDTO>,
) -> impl Responder {
    let me = get_uuid_from_header!(req);
    let mut friend = friend.into_inner();
    friend.accept_user = me;
    let res = process_friend_with_notify(state.db(), friend).await;
    respond_json_any!(res)
}

/// 获取分配给当前用户的外网 QUIC 节点地址(哈希取模,单节点)
#[get("/quic_servers")]
pub async fn get_quic_server_for_user_api(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> impl Responder {
    let uuid = match get_uuid_from_header!(req) {
        Some(uuid) => uuid,
        None => {
            return HttpResponse::Unauthorized()
                .body(CommonResponseNoDataRef::error_json("Unauthorized"));
        }
    };
    respond_json_any!(get_quic_server_for_user(state.redis(), &uuid).await)
}

/// 获取 NAT UDP 端口配置(需登录)
#[get("/nat_udp_ports")]
pub async fn get_nat_udp_ports_api(state: web::Data<AppState>, req: HttpRequest) -> impl Responder {
    let _uuid = match get_uuid_from_header!(req) {
        Some(uuid) => uuid,
        None => {
            return HttpResponse::Unauthorized()
                .body(CommonResponseNoDataRef::error_json("Unauthorized"));
        }
    };
    respond_json_any!(get_nat_udp_ports(state.redis()).await)
}
