use anyhow::anyhow;
use crate::module::user_mod::entity::friend::{Friend, FriendLink, FriendLinkInfo};
use crate::module::user_mod::vo::friend_vo::{query_friend_list, FriendVO};
use crate::utils::http_response::{CommonResponseNoDataRef, CommonResponseRef};
use crate::utils::time::get_now_time_stamp_as_millis;
use rbatis::{Error, RBatis};
use crate::module::user_mod::entity::basic_user::BasicUser;

pub async fn get_friend_by_id(rb: &RBatis) -> Result<String, anyhow::Error> {
    let friend: Result<Vec<Friend>, Error> = Friend::select_by_column(rb, "uuid", "123").await;

    let friend_vo = FriendVO {
        uuid: "friend not found",
    };

    let common_response_ref = CommonResponseRef::<FriendVO>::success_json(&friend_vo);

    Ok(common_response_ref?)
}

///发起好友申请
pub async fn add_friend(
    rb: &RBatis,
    request_user: Option<String>,
    accept_user: Option<String>,
) -> Result<String, anyhow::Error> {
    let uuid = uuid::Uuid::now_v7().to_string();
    // 开启事务
    let mut tx = rb.acquire_begin().await?;

    // 使用事务块包裹逻辑
    let result = async {
        let friend_link = FriendLink {
            uuid: Some(uuid.parse()?),
            request_user,
            accept_user,
            enable: Some(false)
        };

        FriendLink::insert(rb, &friend_link).await?;

        let now = get_now_time_stamp_as_millis()?;
        let friend_link_info: FriendLinkInfo = FriendLinkInfo {
            uuid: friend_link.uuid,
            accept_status: Some(0),
            create_at: Some(now),
            update_at: Some(now),
            request_message: None
        };

        FriendLinkInfo::insert(rb, &friend_link_info).await?;
        tx.commit().await?;
        Ok(CommonResponseNoDataRef::success_empty())
    }
    .await;
    // 如果事务中有错误，回滚事务
    if result.is_err() {
        let _ = tx.rollback().await;
    }
    result
}

pub async fn agree_friend_request() -> Result<String, anyhow::Error> {
  Ok(CommonResponseNoDataRef::success_empty())
}

pub async fn get_friend_list(rb: &RBatis, request_user: Option<String>) -> Result<String, anyhow::Error> {
    let account = request_user.ok_or(anyhow!("获取账号失败!"))?;
    let mut basic_user = BasicUser::select_by_column(rb, "account", &account).await?;
    let user = match basic_user.get(0) {
        None => {return Err(anyhow!("好友列表空"))}
        Some(_) => {basic_user.remove(0)}
    };
    let account = user.uuid.ok_or(anyhow!("获取账号失败!"))?;
    query_friend_list(rb, &account).await
}
