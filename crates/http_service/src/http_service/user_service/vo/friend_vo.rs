use rbatis::RBatis;
use rbatis::rbdc::Uuid;
use rbs::value;
use serde::{Deserialize, Serialize};

use crate::utils::http_response::CommonResponseRef;

#[derive(Serialize, Deserialize, Debug)]
pub struct FriendListVO {
    pub uuid: Option<String>,
    pub account: Option<String>,
    pub username: Option<String>,
    pub icon: Option<String>,
    pub info: Option<String>,
    pub is_del: Option<bool>,
    pub is_block: Option<bool>,
    pub version: Option<i32>,
    pub updated_at: Option<i64>,
    pub created_at: Option<i64>,
}

pub async fn query_friend_list(
    rb: &RBatis,
    uuid: &Uuid,
    created_at: i64,
) -> Result<String, anyhow::Error> {
    let friend_list: Option<Vec<FriendListVO>> = rb
        .query_decode("select bu.uuid, bu.username, bu.account, bu.icon, bu.info, fs.is_del, fs.is_block, fs.version, fs.updated_at, fs.created_at from
(select accept_user as uuid, is_del, (bl.uuid is not null) as is_block, updated_at, version, created_at FROM friend_link
left join black_list bl on bl.me_user = ? and bl.block_user = accept_user
where request_user = ?
union all
select request_user as uuid, is_del, (bl.uuid is not null) as is_block, updated_at, version, created_at FROM friend_link
left join black_list bl on bl.me_user = ? and bl.block_user = request_user
where accept_user = ? ) fs left join basic_user bu
on fs.uuid = bu.uuid where fs.updated_at >= ?", vec![value!(uuid), value!(uuid), value!(uuid), value!(uuid), value!(created_at)])
        .await?;
    Ok(CommonResponseRef::<Option<Vec<FriendListVO>>>::success_json(&friend_list)?)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BlackListVO {
    pub uuid: Option<String>,
    pub account: Option<String>,
    pub username: Option<String>,
    pub icon: Option<String>,
    pub created_at: Option<i64>,
}

pub async fn query_black_list(rb: &RBatis, uuid: &Uuid) -> Result<String, anyhow::Error> {
    let black_list: Option<Vec<BlackListVO>> = rb
        .query_decode("select bu.uuid, bu.account, bu.username, bu.icon, bl.created_at from black_list bl left join basic_user bu on bl.block_user = bu.uuid where bl.me_user = ?", vec![value!(uuid)])
        .await?;
    Ok(CommonResponseRef::<Option<Vec<BlackListVO>>>::success_json(&black_list)?)
}
