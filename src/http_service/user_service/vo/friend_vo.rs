use crate::utils::http_response::CommonResponseRef;
use rbatis::RBatis;
use serde::{Deserialize, Serialize};
use rbatis::rbdc::Uuid;
use rbs::value;

#[derive(Serialize, Debug)]
pub struct FriendVO<'a> {
    pub uuid: &'a str,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FriendListVO {
    pub uuid: Option<String>,
    pub account: Option<String>,
    pub username: Option<String>,
    pub icon: Option<String>,
    pub info: Option<String>,
    pub is_del: Option<bool>,
    pub version: Option<i32>,
    pub updated_at: Option<i64>,
    pub created_at: Option<i64>,
}

pub async fn query_friend_list(rb: &RBatis, uuid: &Uuid, created_at: i64) -> Result<String, anyhow::Error> {
    let friend_list: Option<Vec<FriendListVO>> = rb
        .query_decode("select bu.uuid, bu.username, bu.account, bu.icon, bu.info, fs.is_del, fs.version, fs.updated_at, fs.created_at from
(select accept_user as uuid, is_del, updated_at, version, created_at FROM friend_link
where request_user = ? 
union all
select request_user as uuid, is_del, updated_at, version, created_at FROM friend_link
where accept_user = ? ) fs left join basic_user bu
on fs.uuid = bu.uuid where fs.updated_at >= ?", vec![value!(uuid), value!(uuid), value!(created_at)])
        .await?;
    Ok(CommonResponseRef::<Option<Vec<FriendListVO>>>::success_json(&friend_list)?)
}
