use crate::utils::http_response::CommonResponseRef;
use actix_web::web;
use anyhow::anyhow;
use rbatis::RBatis;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use log::info;
use rbatis::rbdc::Uuid;

#[derive(Serialize, Debug)]
pub struct FriendVO<'a> {
    pub uuid: &'a str,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FriendListVO {
    pub account: Option<String>,
    pub username: Option<String>,
    pub icon: Option<String>,
    pub info: Option<String>,
}

pub async fn query_friend_list(rb: &RBatis, uuid: &Uuid) -> Result<String, anyhow::Error> {
    let friend_list: Option<Vec<FriendListVO>> = rb
        .query_decode("select bu.username, bu.account, bu.icon, bu.info from
(select accept_user as uuid FROM friend_link
where request_user = ? and enable = true
union all
select request_user as uuid FROM friend_link
where accept_user = ? and enable = true) fs left join basic_user bu
on fs.uuid = bu.uuid", vec![rbs::to_value!(uuid), rbs::to_value!(uuid)])
        .await?;
    Ok(CommonResponseRef::<Option<Vec<FriendListVO>>>::success_json(&friend_list)?)
}
