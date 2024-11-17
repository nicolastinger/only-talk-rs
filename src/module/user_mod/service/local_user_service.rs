use std::future::Future;
use actix_web::{web, HttpResponse};
use log::{error, info};
use rbatis::RBatis;
use rbatis::rbdc::db::ExecResult;
use rbs::Error;
use crate::module::user_mod::model::basic_user::{get_raw_sql, BasicUser};

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
    info!("1 {:?}", basic_user_all);
    info!("2 {:?}", basic_user_icon);
    info!("3 {:?}", basic_user_all_id);
    basic_user_all
}

pub async fn get_exit_user (rb: &RBatis,account:String) -> bool{
    match BasicUser::select_by_account(rb, account).await {
        Ok(user) => user.is_some(),
        Err(error) => {
            error!("查询用户是否存在出错 {}", error);
            true
        }
    }
}

pub async fn add_new_basic_user_service (rb: &RBatis,basic_user: BasicUser) -> Result<String,String>{
    match get_exit_user(rb, basic_user.account.clone().unwrap()).await {
        true => {
          Err("该账号已存在!".parse().unwrap())
        }
        false => {
           match  BasicUser::insert(rb,&basic_user).await {
               Ok(_) => {
                   Ok("新增账号成功!".to_string())
               }
               Err(_) => {
                   Err("新增账号失败!".parse().unwrap())
               }
           }
        }
    }
}