use crate::http_service::user_service::dto::friend_request_info_dto::FriendRequestInfoDTO;
use crate::http_service::user_service::vo::friend_vo::{query_friend_list, FriendVO};
use anyhow::anyhow;
use rbatis::RBatis;
use rbs::value;
use serde_json::json;
use std::str::FromStr;
use uuid::Uuid;
use entity::models::user_entity::basic_user::is_exist_user_by_uuid;
use entity::models::user_entity::friend_link::FriendLink;
use entity::models::user_entity::friend_request_info::FriendRequestInfo;
use entity::utils::time::get_now_time_stamp_as_millis;
use crate::utils::http_response::{CommonResponseNoDataRef, CommonResponseRef};

pub async fn get_friend_by_id(rb: &RBatis) -> Result<String, anyhow::Error> {
    let friend_vo = FriendVO {
        uuid: "friend not found",
    };

    let common_response_ref = CommonResponseRef::<FriendVO>::success_json(&friend_vo);

    Ok(common_response_ref?)
}

///发起好友申请
pub async fn add_friend(
    rb: &RBatis,
    friend_request_info_dto: FriendRequestInfoDTO,
) -> Result<String, anyhow::Error> {
    let uuid = Uuid::now_v7().to_string();
    // 开启事务
    let tx = rb.acquire_begin().await?;
    let request_user = friend_request_info_dto
        .request_user
        .ok_or_else(|| anyhow!("request_user is None"))?;
    let request_user = rbatis::rbdc::Uuid::from_str(request_user.as_str())?;
    let accept_user_str = friend_request_info_dto
        .accept_user
        .ok_or(anyhow!("accept_user is None"))?;
    let accept_user = accept_user_str.clone();
    let accept_user = rbatis::rbdc::Uuid::from_str(accept_user.as_str())?;

    // 是否存在这个接受用户
    let is_exist_accept_user = is_exist_user_by_uuid(rb, &accept_user).await?;
    if !is_exist_accept_user {
        return Ok(CommonResponseNoDataRef::error_json("接受用户不存在"));
    }
    // TODO 是否被接受用户拉黑
    // TODO 是否超出发送用户或者接受用户添加数量

    // 查询是否为已添加
    let friend_link = FriendLink::select_by_last_uuid(rb, &request_user, &accept_user).await?;
    if friend_link.is_some() && friend_link.as_ref().unwrap().is_del.unwrap_or(true) == false {
        return Ok(CommonResponseNoDataRef::error_json("已添加"));
    }

    // 查询之前添加的申请
    let friend_request_info =
        FriendRequestInfo::select_by_uuid(rb, &request_user, &accept_user).await?;
    if !friend_request_info.is_empty() {
        for item in friend_request_info.iter() {
            match item.accept_status {
                Some(0) => {
                    return Ok(CommonResponseNoDataRef::error_json("请勿重复添加"));
                }
                _ => {}
            }
        }
    }

    // 使用事务块包裹逻辑
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
        Ok(CommonResponseNoDataRef::success_empty())
    }
    .await;
    // 如果事务中有错误，回滚事务
    if result.is_err() {
        let _ = tx.rollback().await;
    }
    result
}

/// 处理好友申请
pub async fn process_friend(
    rb: &RBatis,
    friend_request_info_dto: FriendRequestInfoDTO,
) -> Result<String, anyhow::Error> {
    let request_user = friend_request_info_dto
        .request_user
        .ok_or_else(|| anyhow!("request_user is None"))?;
    let request_user_str = request_user.clone();
    let request_user = rbatis::rbdc::Uuid::from_str(request_user.as_str())?;
    let accept_user = friend_request_info_dto
        .accept_user
        .ok_or(anyhow!("accept_user is None"))?;
    let accept_user = rbatis::rbdc::Uuid::from_str(accept_user.as_str())?;

    // 是否存在这个发起用户
    let is_exist_accept_user = is_exist_user_by_uuid(rb, &request_user).await?;
    if !is_exist_accept_user {
        return Ok(CommonResponseNoDataRef::error_json("请求用户不存在"));
    }
    // TODO 是否超出接受用户添加数量

    // 查询是否为已添加
    let friend_link = FriendLink::select_by_last_uuid(rb, &request_user, &accept_user).await?;
    if friend_link.is_some() && friend_link.as_ref().unwrap().is_del.unwrap_or(true) == false {
        return Ok(CommonResponseNoDataRef::error_json("已添加"));
    }

    // 查询之前添加的申请
    let friend_request_info =
        FriendRequestInfo::select_by_uuid(rb, &request_user, &accept_user).await?;
    let get_request_info = || {
        for item in friend_request_info {
            match item.accept_status {
                Some(0) => {
                    return Some(item);
                }
                _ => {}
            }
        }
        None
    };
    let mut exit_request_info = get_request_info().ok_or(anyhow!("未找到申请"))?;
    // 开启事务
    let tx = rb.acquire_begin().await?;
    // 使用事务块包裹逻辑
    let result = async {
        let now = get_now_time_stamp_as_millis()?;
        let uuid = exit_request_info
            .uuid
            .clone()
            .ok_or(anyhow!("uuid is None"))?;

        exit_request_info.accept_status = friend_request_info_dto.accept_status;
        exit_request_info.accept_message = friend_request_info_dto.accept_message;
        exit_request_info.version = Some(exit_request_info.version.unwrap_or(0) + 1);
        exit_request_info.updated_at = Some(now);

        let update_value = value! {"id":&exit_request_info.id};
        let data = FriendRequestInfo::update_by_map(rb, &exit_request_info, update_value).await?;
        match friend_request_info_dto.accept_status {
            // 接受
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
            // 拒绝
            Some(2) => {}
            _ => {
                return Ok(CommonResponseNoDataRef::error_json("参数错误"));
            }
        }
        tx.commit().await?;
        println!("update_by_map = {}", json!(data));

        Ok(CommonResponseNoDataRef::success_empty())
    }
    .await;
    // 如果事务中有错误，回滚事务
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
    let uuid = request_user.ok_or(anyhow!("获取账号失败!"))?;
    let version = version.parse::<i32>()?;
    let uuid = rbatis::rbdc::uuid::Uuid::from_str(&uuid)?;

    let uuid_v7 = Uuid::from_str(last_uuid.as_str()).unwrap_or(Uuid::now_v7());

    let uuid_v7 = rbatis::rbdc::uuid::Uuid::from_str(uuid_v7.to_string().as_str())?;

    let mut timestamp = 0i64;

    let res = FriendLink::select_by_last_uuid(rb, &uuid, &uuid_v7).await?;

    if res.is_some()
        && res
            .as_ref()
            .ok_or(anyhow!("获取时间戳失败"))?
            .version
            .unwrap_or_else(|| -1i32)
            == version
    {
        timestamp = res
            .ok_or(anyhow!("获取时间戳失败"))?
            .created_at
            .unwrap_or(0i64);
    }

    query_friend_list(rb, &uuid, timestamp).await
}
