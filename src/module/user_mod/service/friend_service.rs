use std::str::FromStr;
use anyhow::anyhow;
use log::info;
use crate::module::user_mod::entity::friend::{Friend, FriendLink, FriendLinkInfo};
use crate::module::user_mod::vo::friend_vo::{query_friend_list, FriendVO};
use crate::utils::http_response::{CommonResponseNoDataRef, CommonResponseRef};
use crate::utils::time::get_now_time_stamp_as_millis;
use rbatis::{Error, RBatis};
use rbs::value;
use uuid::Uuid;
use crate::module::user_mod::entity::basic_user::BasicUser;
use crate::module::user_mod::service::local_user_service::get_user_uuid_by_account;

pub async fn get_friend_by_id(rb: &RBatis) -> Result<String, anyhow::Error> {
    let friend: Result<Vec<Friend>, Error> = Friend::select_by_map(rb, value!{"uuid" : "123"}).await;

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
    let uuid = Uuid::now_v7().to_string();
    // 开启事务
    let mut tx = rb.acquire_begin().await?;
    let request_user = request_user.ok_or_else(|| anyhow!("request_user is None"))?;
    let request_user = rbatis::rbdc::Uuid::from_str(request_user.as_str())?;
    let accept_user = get_user_uuid_by_account(accept_user.ok_or_else(|| anyhow!("accept_user is None"))?).await?;
    let accept_user = rbatis::rbdc::Uuid::from_str(accept_user.to_string().as_str())?;
    // 使用事务块包裹逻辑
    let result = async {
        let now = get_now_time_stamp_as_millis()?;
        
        let friend_link_info: FriendLinkInfo = FriendLinkInfo {
            uuid: Some(uuid.parse()?),
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

pub async fn get_friend_list(rb: &RBatis, request_user: Option<String>, last_uuid: String, version: String) -> Result<String, anyhow::Error> {
    let uuid = request_user.ok_or(anyhow!("获取账号失败!"))?;
    let version = version.parse::<i32>()?;
    let uuid = rbatis::rbdc::uuid::Uuid::from_str(&uuid)?;

    let uuid_v7 = Uuid::from_str(last_uuid.as_str()).unwrap_or(Uuid::now_v7());

    let uuid_v7 = rbatis::rbdc::uuid::Uuid::from_str(uuid_v7.to_string().as_str())?;

    let mut timestamp = 0i64;


    let res = FriendLink::select_by_last_uuid(rb, &uuid, &uuid_v7).await?;
    info!("last {}",uuid_v7);
    if res.is_some() && res.as_ref().ok_or(anyhow!("获取时间戳失败"))?.version.unwrap_or_else(|| -1i32) == version{
        timestamp = res.ok_or(anyhow!("获取时间戳失败"))?.created_at.unwrap_or(0i64);
    }

    query_friend_list(rb, &uuid, timestamp).await
}
