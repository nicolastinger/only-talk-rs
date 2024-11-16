use std::collections::HashMap;
use actix_web::web;
use rbatis::{crud, impl_delete, impl_select, impl_select_page, impl_update, RBatis};
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct BasicUser {
    pub uuid: Option<String>,
    pub id: Option<i64>,
    pub username: Option<String>,
    pub account: Option<String>,
    pub icon: Option<String>,
    pub info: Option<String>
}

crud!(BasicUser {});  //crud = insert+select_by_column+update_by_column+delete_by_column
impl_select!(BasicUser{select_all_by_id(username:&str,account:&str) => "`where icon = #{username} and info = #{account}`"});
impl_select!(BasicUser{select_by_account(account:String) -> Option => "`where account = #{account} limit 1`"});
impl_update!(BasicUser{update_by_name(name:&str) => "`where id = 1`"});
impl_delete!(BasicUser {delete_by_name(name:&str) => "`where name= '2'`"});
impl_select_page!(BasicUser{select_page(name:&str) => "`where name != #{name}`"});

pub async fn get_raw_sql(rb: web::Data<RBatis>) {
    let table: Option<Vec<HashMap<String,serde_json::Value>>> = rb
        .query_decode("select * from rust_user_test where id = ? limit ?", vec![rbs::to_value!("huangjinsheng"), rbs::to_value!(1)])
        .await
        .unwrap();
    if let Some(t) = table {
        for i in t.iter() {
            for (k,v) in i.iter() {
                println!("{}: {}", k, serde_json::to_string_pretty(&v).unwrap());
            }
        }
    }
}

#[derive(Clone, Deserialize, Serialize, Debug)]
struct UserInfo {
    username: Option<String>,
    account: Option<String>,
    icon: Option<String>,  //头像的base64字符串
    password: Option<String>,
    gender: Option<u8>,  //0-m,1-s,2-男,3-女,4-机器人,5-other,etc
    age: Option<u8>,
    info: Option<String>,  //简介
    create_time: Option<i64>,
    update_time: Option<i64>,
    last_login_time: Option<i64>,
    last_login_equipment: Option<String>,  //最后登录的设备
    last_login_ipv4: Option<String>,
    last_login_ipv6: Option<String>,
    phone: Option<String>,
    email: Option<String>,
    address: Option<String>,
    is_active: Option<bool>
}