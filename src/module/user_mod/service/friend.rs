use rbatis::{Error, RBatis};
use serde::Serialize;
use uuid::{NoContext, Timestamp};
use crate::module::user_mod::model::friend::{Friend, FriendLink};
use crate::module::user_mod::vo::friend_vo::FriendVO;
use crate::serde_json_to_string;
use crate::utils::http_response::CommonResponseRef;

pub async fn get_friend_by_id(rb: &RBatis) -> Result<String, anyhow::Error> {
    let friend: Result<Vec<Friend>, Error> = Friend::select_by_column(rb, "uuid", "123").await;

    let friend_vo = FriendVO{
        uuid: "friend not found",
    };

    let common_response_ref = CommonResponseRef::<FriendVO>::success_json(&friend_vo);

    Ok(common_response_ref?)
}


pub async fn fn_add_friend(rb: &RBatis) -> Result<String, anyhow::Error> {
    let friend_link = FriendLink{
        uuid: Some(uuid::Uuid::now_v7()),
        request_user: Some("caixukun".to_string()),
        accept_user: Some("huangxiaoming".to_string()),
        enable: Some(true)
    };
    log::info!("friend link {:?}", friend_link);
    FriendLink::insert(rb, &friend_link).await?;
    Ok(format!("Added friend with id {:?}", friend_link.uuid))
}