use actix_web::{post, HttpResponse, Responder};

/// 授权某用户查看自己的udp地址
#[post("/auth_user_check_udp/{account}")]
pub async fn add_user_udp_addr_by_account_api() -> impl Responder {
  // TODO
    HttpResponse::Ok()
}

#[post("/get_user_udp_addr/{account}")]
pub async fn get_user_udp_addr_by_account_api() -> impl Responder {
    HttpResponse::Ok()
}