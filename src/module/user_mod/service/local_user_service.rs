use actix_web::web;
use rbatis::RBatis;
use crate::module::user_mod::model::basic_user::get_raw_sql;

pub async fn get_user_raw (rb: web::Data<RBatis>) {
    get_raw_sql(rb).await
}

pub async fn create_new_user (rb: web::Data<RBatis>) {
    get_raw_sql(rb).await
}