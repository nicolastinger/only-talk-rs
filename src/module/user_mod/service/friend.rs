use rbatis::{Error, RBatis};
use serde::Serialize;
use crate::module::user_mod::model::friend::Friend;
use crate::module::user_mod::vo::friend_vo::FriendVO;
use crate::serde_json_to_string;
use crate::utils::http_response::CommonResponseRef;

pub async fn get_friend_by_id(rb: &RBatis) -> Result<String, String> {
    let friend: Result<Vec<Friend>, Error> = Friend::select_by_column(rb, "uuid", "123").await;

    let friend_vo = FriendVO{
        uuid: "friend not found",
    };

    let common_response_ref = CommonResponseRef::<FriendVO>::success_json(&friend_vo);

    common_response_ref
}