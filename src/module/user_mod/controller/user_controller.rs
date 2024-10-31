use actix_web::{get, post, HttpResponse, Responder};

#[get("/user_test")]
pub async fn user_test() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/plain; charset=utf-8")
        .body("我真是蔡徐坤")
}

#[post("/user_test")]
pub async fn post_test() -> impl Responder {
    HttpResponse::Ok().body("蔡徐坤")
}