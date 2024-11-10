use std::collections::HashMap;
use actix_web::web;
use rbatis::RBatis;

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
