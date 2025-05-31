use actix_web::{post, HttpResponse, Responder};

#[post("/get_chat_record/{account}")]
pub async fn get_chat_record () -> impl Responder {
    HttpResponse::Ok().body("<!-- chatRecord -->")
}