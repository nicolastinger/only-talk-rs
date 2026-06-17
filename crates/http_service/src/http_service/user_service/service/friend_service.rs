use std::str::FromStr;

use anyhow::anyhow;
use common::models::user_entity::basic_user::is_exist_user_by_uuid;
use common::models::user_entity::friend_link::FriendLink;
use common::models::user_entity::friend_request_info::FriendRequestInfo;
use common::utils::time::get_now_time_stamp_as_millis;
use rbatis::RBatis;
use rbs::value;
use uuid::Uuid;

use crate::http_service::user_service::dto::friend_request_info_dto::FriendRequestInfoDTO;
use crate::http_service::user_service::vo::friend_vo::query_friend_list;
use crate::utils::http_response::CommonResponseRef;

/// Initiate friend request
pub async fn add_friend(
    rb: &RBatis,
    friend_request_info_dto: FriendRequestInfoDTO,
) -> Result<FriendRequestInfo, anyhow::Error> {
    let uuid = Uuid::now_v7().to_string();
    // Start transaction
    let tx = rb.acquire_begin().await?;
    let request_user =
        friend_request_info_dto.request_user.ok_or_else(|| anyhow!("request_user is None"))?;
    let request_user = rbatis::rbdc::Uuid::from_str(request_user.as_str())?;
    let accept_user_str =
        friend_request_info_dto.accept_user.ok_or(anyhow!("accept_user is None"))?;
    let accept_user = accept_user_str.clone();
    let accept_user = rbatis::rbdc::Uuid::from_str(accept_user.as_str())?;

    // Check if accept user exists
    let is_exist_accept_user = is_exist_user_by_uuid(rb, &accept_user).await?;
    if !is_exist_accept_user {
        return Err(anyhow!("Accept user does not exist"));
    }
    // TODO Check if blocked by accept user
    // TODO Check if sender/receiver friend request limit exceeded

    // Check if already added
    let friend_link = FriendLink::select_by_last_uuid(rb, &request_user, &accept_user).await?;
    if friend_link.is_some()
        && !friend_link.as_ref().ok_or(anyhow!("friend_link is None"))?.is_del.unwrap_or(true)
    {
        return Err(anyhow!("Already added as friend"));
    }

    // Query previous friend requests
    let friend_request_info =
        FriendRequestInfo::select_by_uuid(rb, &request_user, &accept_user).await?;
    if !friend_request_info.is_empty() {
        for item in friend_request_info.iter() {
            if let Some(0) = item.accept_status {
                return Err(anyhow!("Please do not add repeatedly"));
            }
        }
    }

    // Wrap logic in transaction block
    let result = async {
        let now = get_now_time_stamp_as_millis()?;

        let friend_link_info: FriendRequestInfo = FriendRequestInfo {
            id: None,
            uuid: Some(uuid.parse()?),
            accept_status: Some(0),
            created_at: Some(now),
            updated_at: Some(now),
            request_message: friend_request_info_dto.request_message,
            accept_message: None,
            request_user: Some(request_user),
            accept_user: Some(accept_user),
            add_type: friend_request_info_dto.add_type,
            version: Some(0),
        };

        FriendRequestInfo::insert(rb, &friend_link_info).await?;
        tx.commit().await?;
        Ok(friend_link_info)
    }
    .await;
    // Rollback transaction on error
    if result.is_err() {
        let _ = tx.rollback().await;
    }
    result
}

/// Process friend request
pub async fn process_friend(
    rb: &RBatis,
    friend_request_info_dto: FriendRequestInfoDTO,
) -> Result<FriendRequestInfo, anyhow::Error> {
    let request_user =
        friend_request_info_dto.request_user.ok_or_else(|| anyhow!("request_user is None"))?;
    let request_user = rbatis::rbdc::Uuid::from_str(request_user.as_str())?;
    let accept_user = friend_request_info_dto.accept_user.ok_or(anyhow!("accept_user is None"))?;
    let accept_user = rbatis::rbdc::Uuid::from_str(accept_user.as_str())?;

    // Check if request user exists
    let is_exist_accept_user = is_exist_user_by_uuid(rb, &request_user).await?;
    if !is_exist_accept_user {
        return Err(anyhow!("Request user does not exist"));
    }
    // TODO Check if accept user friend limit exceeded

    // Check if already added
    let friend_link = FriendLink::select_by_last_uuid(rb, &request_user, &accept_user).await?;
    if friend_link.is_some()
        && !friend_link.as_ref().ok_or(anyhow!("friend_link is None"))?.is_del.unwrap_or(true)
    {
        return Err(anyhow!("Already added as friend"));
    }

    // Query previous friend requests
    let friend_request_info =
        FriendRequestInfo::select_by_uuid(rb, &request_user, &accept_user).await?;
    let get_request_info = || {
        for item in friend_request_info {
            if let Some(0) = item.accept_status {
                return Some(item);
            }
        }
        None
    };
    let mut exit_request_info = get_request_info().ok_or(anyhow!("Request not found"))?;
    // Start transaction
    let tx = rb.acquire_begin().await?;
    // Wrap logic in transaction block
    let result = async {
        let now = get_now_time_stamp_as_millis()?;
        let uuid = exit_request_info.uuid.clone().ok_or(anyhow!("uuid is None"))?;

        exit_request_info.accept_status = friend_request_info_dto.accept_status;
        exit_request_info.accept_message = friend_request_info_dto.accept_message;
        exit_request_info.version = Some(exit_request_info.version.unwrap_or(0) + 1);
        exit_request_info.updated_at = Some(now);

        let update_value = value! {"id":&exit_request_info.id};
        FriendRequestInfo::update_by_map(rb, &exit_request_info, update_value).await?;
        match friend_request_info_dto.accept_status {
            // Accept
            Some(1) => {
                let friend_link = FriendLink {
                    uuid: Some(uuid),
                    request_user: Some(request_user),
                    accept_user: Some(accept_user),
                    is_del: Some(false),
                    created_at: Some(now),
                    updated_at: Some(now),
                    version: Some(0),
                };
                FriendLink::insert(rb, &friend_link).await?;
            }
            // Reject
            Some(2) => {}
            _ => {
                return Err(anyhow!("Invalid parameter"));
            }
        }
        tx.commit().await?;

        Ok(exit_request_info)
    }
    .await;
    // Rollback transaction on error
    if result.is_err() {
        let _ = tx.rollback().await;
    }
    result
}

pub async fn get_friend_list(
    rb: &RBatis,
    request_user: Option<String>,
    last_uuid: String,
    version: String,
) -> Result<String, anyhow::Error> {
    let uuid = request_user.ok_or(anyhow!("Failed to get account!"))?;
    let version = version.parse::<i32>()?;
    let uuid = rbatis::rbdc::uuid::Uuid::from_str(&uuid)?;

    let uuid_v7 = Uuid::from_str(last_uuid.as_str()).unwrap_or(Uuid::now_v7());

    let uuid_v7 = rbatis::rbdc::uuid::Uuid::from_str(uuid_v7.to_string().as_str())?;

    let mut timestamp = 0i64;

    let res = FriendLink::select_by_last_uuid(rb, &uuid, &uuid_v7).await?;

    if res.is_some()
        && res.as_ref().ok_or(anyhow!("Failed to get timestamp!"))?.version.unwrap_or(-1i32)
            == version
    {
        timestamp = res.ok_or(anyhow!("Failed to get timestamp!"))?.created_at.unwrap_or(0i64);
    }

    query_friend_list(rb, &uuid, timestamp).await
}

/// Get list of friend requests I received
pub async fn get_accept_friend_request_list(
    rb: &RBatis,
    uuid: Option<String>,
    accept_status: Option<u8>,
) -> Result<String, anyhow::Error> {
    let uuid = uuid.ok_or(anyhow!("Failed to get account"))?;
    let uuid = rbatis::rbdc::uuid::Uuid::from_str(&uuid)?;

    let res = FriendRequestInfo::select_by_accept_user_and_status(rb, &uuid, accept_status).await?;
    Ok(CommonResponseRef::<Vec<FriendRequestInfo>>::success_json(&res)?)
}

/// Get list of friend requests I sent
pub async fn get_friend_request_list(
    rb: &RBatis,
    uuid: Option<String>,
    accept_status: Option<u8>,
) -> Result<String, anyhow::Error> {
    let uuid = uuid.ok_or(anyhow!("Failed to get account"))?;
    let uuid = rbatis::rbdc::uuid::Uuid::from_str(&uuid)?;

    let res =
        FriendRequestInfo::select_by_request_user_and_status(rb, &uuid, accept_status).await?;
    Ok(CommonResponseRef::<Vec<FriendRequestInfo>>::success_json(&res)?)
}

/// Delete friend
pub async fn delete_friend_service(
    rb: &RBatis,
    my_uuid: Option<String>,
    friend_uuid: String,
) -> Result<String, anyhow::Error> {
    let my_uuid_str = my_uuid.ok_or(anyhow!("Failed to get current user ID"))?;
    let my_uuid = rbatis::rbdc::Uuid::from_str(&my_uuid_str)?;
    let friend_uuid = rbatis::rbdc::Uuid::from_str(&friend_uuid)?;

    let friend_link = FriendLink::select_by_last_uuid(rb, &my_uuid, &friend_uuid).await?;
    if friend_link.is_none() {
        return Err(anyhow!("Friend relationship does not exist"));
    }
    let mut friend_link = friend_link.ok_or(anyhow!("Friend relationship does not exist"))?;
    if friend_link.is_del.unwrap_or(true) {
        return Err(anyhow!("Friend relationship has been deleted"));
    }

    let now = get_now_time_stamp_as_millis()?;
    friend_link.is_del = Some(true);
    friend_link.updated_at = Some(now);
    friend_link.version = Some(friend_link.version.unwrap_or(0) + 1);

    FriendLink::update_is_del_by_users(rb, &friend_link, &my_uuid, &friend_uuid).await?;

    Ok(crate::utils::http_response::CommonResponseNoDataRef::success_empty())
}
