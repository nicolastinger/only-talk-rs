use actix_web::{web, HttpResponse};
use log::info;
use rbatis::RBatis;
use crate::module::user_mod::model::basic_user::{get_raw_sql,BasicUser};

pub async fn get_user_raw (rb: web::Data<RBatis>) {
    get_raw_sql(rb).await
}

pub async fn create_new_user (rb: web::Data<RBatis>) {
    get_raw_sql(rb).await
}

pub async fn test_sql (rb: &RBatis) -> Vec<BasicUser>{
    let basic_user_all = BasicUser::select_all(rb).await.unwrap();
    let basic_user_icon = BasicUser::select_by_column(rb,"icon","33333").await.unwrap();
    let basic_user_all_id = BasicUser::select_all_by_id(rb,"33333","4444444").await.unwrap();
    let select_by_id = BasicUser::select_by_id(rb, "33333".into()).await.unwrap();
    info!("1 {:?}", basic_user_all);
    info!("2 {:?}", basic_user_icon);
    info!("3 {:?}", basic_user_all_id);
    info!("4 {:?}", select_by_id.unwrap());
    basic_user_all
}