use std::str::FromStr;

use anyhow::anyhow;
use common::models::chat_entity::chat_message_record::ChatMessageRecord;
use common::models::session_entity::session::{SESSION_TYPE_SINGLE, Session};
use common::models::session_entity::user_session::UserSession;
use common::models::user_entity::basic_user::is_exist_user_by_uuid;
use common::models::user_entity::black_list::BlackList;
use common::models::user_entity::friend_link::FriendLink;
use common::models::user_entity::friend_request_info::FriendRequestInfo;
use common::utils::session_uuid::single_session_uuid;
use common::utils::time::get_now_time_stamp_as_millis;
use nanoid::nanoid;
use rbatis::RBatis;
use rbatis::executor::Executor;
use rbatis::rbdc::{Bytes, Uuid as RbdcUuid};
use rbs::value;
use uuid::Uuid;

use crate::http_service::user_service::dto::friend_request_info_dto::FriendRequestInfoDTO;
use crate::http_service::user_service::vo::friend_vo::{query_black_list, query_friend_list};
use crate::utils::http_response::CommonResponseRef;

/// 申请留言缺省文案(申请时未填)。
pub const FRIEND_REQUEST_DEFAULT_MSG: &str = "我请求添加你为好友";
/// 通过提示缺省文案(通过时未填留言)。
pub const FRIEND_ACCEPT_DEFAULT_MSG: &str = "我通过了你的好友申请，现在可以开始聊天了";

/// 发起好友请求
pub async fn add_friend(
    rb: &RBatis,
    friend_request_info_dto: FriendRequestInfoDTO,
) -> Result<FriendRequestInfo, anyhow::Error> {
    let uuid = Uuid::now_v7().to_string();
    // 开启事务
    let tx = rb.acquire_begin().await?;
    let request_user =
        friend_request_info_dto.request_user.ok_or_else(|| anyhow!("request_user is None"))?;
    let request_user = rbatis::rbdc::Uuid::from_str(request_user.as_str())?;
    let accept_user_str =
        friend_request_info_dto.accept_user.ok_or(anyhow!("accept_user is None"))?;
    let accept_user = accept_user_str.clone();
    let accept_user = rbatis::rbdc::Uuid::from_str(accept_user.as_str())?;

    // 检查接收方用户是否存在
    let is_exist_accept_user = is_exist_user_by_uuid(rb, &accept_user).await?;
    if !is_exist_accept_user {
        return Err(anyhow!("Accept user does not exist"));
    }
    // 检查是否被接收方拉黑
    let blocked = is_blocked(rb, &accept_user, &request_user).await?;
    if blocked {
        return Err(anyhow!("You have been blocked by the accept user"));
    }
    // TODO 检查发起/接收方好友请求是否超限

    // 检查是否已添加为好友
    let friend_link = FriendLink::select_by_last_uuid(rb, &request_user, &accept_user).await?;
    if friend_link.is_some()
        && !friend_link.as_ref().ok_or(anyhow!("friend_link is None"))?.is_del.unwrap_or(true)
    {
        return Err(anyhow!("Already added as friend"));
    }

    // 查询历史好友请求
    let friend_request_info =
        FriendRequestInfo::select_by_uuid(rb, &request_user, &accept_user).await?;
    if !friend_request_info.is_empty() {
        for item in friend_request_info.iter() {
            if let Some(0) = item.accept_status {
                return Err(anyhow!("Please do not add repeatedly"));
            }
        }
    }

    // 将逻辑包裹在事务块中
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
    // 出错时回滚事务
    if result.is_err() {
        let _ = tx.rollback().await;
    }
    result
}

/// 处理好友请求
pub async fn process_friend(
    rb: &RBatis,
    friend_request_info_dto: FriendRequestInfoDTO,
) -> Result<FriendRequestInfo, anyhow::Error> {
    let request_user =
        friend_request_info_dto.request_user.ok_or_else(|| anyhow!("request_user is None"))?;
    let request_user = rbatis::rbdc::Uuid::from_str(request_user.as_str())?;
    let accept_user = friend_request_info_dto.accept_user.ok_or(anyhow!("accept_user is None"))?;
    let accept_user = rbatis::rbdc::Uuid::from_str(accept_user.as_str())?;

    // 检查请求方用户是否存在
    let is_exist_accept_user = is_exist_user_by_uuid(rb, &request_user).await?;
    if !is_exist_accept_user {
        return Err(anyhow!("Request user does not exist"));
    }
    // 检查接收方是否已被拉黑
    let blocked = is_blocked(rb, &accept_user, &request_user).await?;
    if blocked {
        return Err(anyhow!("You have been blocked by the accept user"));
    }
    // TODO 检查接收方好友数量是否超限

    // 检查是否已添加为好友
    let existing_friend_link =
        FriendLink::select_by_last_uuid(rb, &request_user, &accept_user).await?;
    if existing_friend_link.is_some()
        && !existing_friend_link
            .as_ref()
            .ok_or(anyhow!("friend_link is None"))?
            .is_del
            .unwrap_or(true)
    {
        return Err(anyhow!("Already added as friend"));
    }

    // 查询历史好友请求
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
    // 开启事务
    let tx = rb.acquire_begin().await?;
    // 将逻辑包裹在事务块中
    let result = async {
        let now = get_now_time_stamp_as_millis()?;
        let uuid = exit_request_info.uuid.clone().ok_or(anyhow!("uuid is None"))?;

        exit_request_info.accept_status = friend_request_info_dto.accept_status;
        exit_request_info.accept_message = friend_request_info_dto.accept_message;
        exit_request_info.version = Some(exit_request_info.version.unwrap_or(0) + 1);
        exit_request_info.updated_at = Some(now);

        let update_value = value! {"id":&exit_request_info.id};
        FriendRequestInfo::update_by_map(&tx, &exit_request_info, update_value).await?;
        match friend_request_info_dto.accept_status {
            // 接受
            Some(1) => {
                // 缺陷S: 删好友是软删(is_del=true), 重加时同 (request_user, accept_user) 会撞
                // friend_link_unique —— 已有软删行则原地复活, 否则新建。
                match &existing_friend_link {
                    Some(link) => {
                        let mut link = link.clone();
                        link.is_del = Some(false);
                        link.updated_at = Some(now);
                        link.version = Some(link.version.unwrap_or(0) + 1);
                        FriendLink::update_by_map(
                            &tx,
                            &link,
                            value! {"request_user": &request_user, "accept_user": &accept_user},
                        )
                        .await?;
                    }
                    None => {
                        let new_link = FriendLink {
                            uuid: Some(uuid),
                            request_user: Some(request_user.clone()),
                            accept_user: Some(accept_user.clone()),
                            is_del: Some(false),
                            created_at: Some(now),
                            updated_at: Some(now),
                            version: Some(0),
                        };
                        FriendLink::insert(&tx, &new_link).await?;
                    }
                }

                // 任务04b: 会话预创建 + 两条前置消息(同事务, 与好友关系成立原子)
                create_session_on_friend_accept(
                    &tx,
                    &request_user,
                    &accept_user,
                    exit_request_info.request_message.as_deref(),
                    exit_request_info.accept_message.as_deref(),
                    now,
                )
                .await?;
            }
            // 拒绝
            Some(2) => {}
            _ => {
                return Err(anyhow!("Invalid parameter"));
            }
        }
        tx.commit().await?;

        Ok(exit_request_info)
    }
    .await;
    // 出错时回滚事务
    if result.is_err() {
        let _ = tx.rollback().await;
    }
    result
}

/// 好友通过即建会话(任务04b)。
///
/// `session` + 双方 `user_session` + 两条前置消息(A 的申请留言 + B 的通过提示)。
/// 幂等源 = 调用方(请求状态迁移恰好一次), 本函数自身可重复执行无守卫。
async fn create_session_on_friend_accept(
    rb: &dyn Executor,
    request_user: &RbdcUuid,
    accept_user: &RbdcUuid,
    request_message: Option<&str>,
    accept_message: Option<&str>,
    now: i64,
) -> Result<(), anyhow::Error> {
    // rbdc::Uuid 是 struct Uuid(String) 新类型(非 uuid::Uuid re-export), 走 to_string/parse 边界转换
    let session_uuid = single_session_uuid(
        &uuid::Uuid::parse_str(&request_user.to_string())?,
        &uuid::Uuid::parse_str(&accept_user.to_string())?,
    );
    let su: RbdcUuid = session_uuid.to_string().parse()?;

    // 1. session 行(幂等: 已存在则 DO NOTHING)
    Session::upsert(
        rb,
        &Session {
            session_uuid: su.clone(),
            session_type: Some(SESSION_TYPE_SINGLE),
            last_message_id: None,
            last_message_at: None,
            last_preview: None,
            created_at: None,
            updated_at: None,
        },
    )
    .await?;

    // 2. 双方 user_session(幂等 upsert; 游标留 None → 落库 0)
    //    ⚠️ 用裸 upsert, 不用 init_for_group_join —— 群聊入群游标推到 max(历史不算未读),
    //    单聊的前置消息本身就是"我们成为好友了"的通知, 必须保留未读。语义相反, 勿"顺手统一"。
    UserSession::upsert(
        rb,
        &UserSession {
            id: None,
            user_uuid: request_user.clone(),
            session_uuid: su.clone(),
            session_type: Some(SESSION_TYPE_SINGLE),
            peer_uuid: Some(accept_user.clone()),
            last_read_id: None,
            synced_id: None,
            pinned: None,
            muted: None,
            deleted_at: None,
            created_at: None,
            updated_at: None,
        },
    )
    .await?;
    UserSession::upsert(
        rb,
        &UserSession {
            id: None,
            user_uuid: accept_user.clone(),
            session_uuid: su.clone(),
            session_type: Some(SESSION_TYPE_SINGLE),
            peer_uuid: Some(request_user.clone()),
            last_read_id: None,
            synced_id: None,
            pinned: None,
            muted: None,
            deleted_at: None,
            created_at: None,
            updated_at: None,
        },
    )
    .await?;

    // 3. 两条前置消息(每次接受都写, 内容取本次申请)
    let msg_a = request_message.filter(|s| !s.is_empty()).unwrap_or(FRIEND_REQUEST_DEFAULT_MSG);
    let msg_b = accept_message.filter(|s| !s.is_empty()).unwrap_or(FRIEND_ACCEPT_DEFAULT_MSG);
    // 3.1 申请留言: A → B
    ChatMessageRecord::insert(
        rb,
        &ChatMessageRecord {
            id: None,
            session_uuid: su.clone(),
            nano_id: Some(nanoid!()),
            timestamp: Some(now),
            raw: Bytes::from(msg_a.as_bytes().to_vec()),
            text_type: Some(0),
            send_user: request_user.clone(),
            recv_user: accept_user.clone(),
        },
    )
    .await?;
    // 3.2 通过提示: B → A(后写, id 更大 —— 会话排序即"对话顺序")
    ChatMessageRecord::insert(
        rb,
        &ChatMessageRecord {
            id: None,
            session_uuid: su.clone(),
            nano_id: Some(nanoid!()),
            timestamp: Some(now),
            raw: Bytes::from(msg_b.as_bytes().to_vec()),
            text_type: Some(0),
            send_user: accept_user.clone(),
            recv_user: request_user.clone(),
        },
    )
    .await?;

    // 4. 会话摘要指向 B 的消息(最新一条)。insert 不回填 bigserial id,
    //    取会话最大 id(单分区索引 top-1, 任务04 既有方法) —— 即 B 条的 id
    let max_id = ChatMessageRecord::max_id_by_session(rb, &su).await?;
    Session::update_last_message(rb, &su, max_id, now, Some(msg_b)).await?;
    Ok(())
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

/// 获取我收到的好友请求列表
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

/// 获取我发出的好友请求列表
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

/// 删除好友
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

/// 检查 me_uuid 是否已被 other_uuid 拉黑
pub async fn is_blocked(
    rb: &RBatis,
    me_uuid: &rbatis::rbdc::Uuid,
    other_uuid: &rbatis::rbdc::Uuid,
) -> Result<bool, anyhow::Error> {
    let res = BlackList::select_by_pair(rb, other_uuid, me_uuid).await?;
    Ok(res.is_some())
}

/// 拉黑用户
pub async fn block_friend_service(
    rb: &RBatis,
    my_uuid: Option<String>,
    friend_uuid: String,
) -> Result<String, anyhow::Error> {
    let my_uuid_str = my_uuid.ok_or(anyhow!("Failed to get current user ID"))?;
    let my_uuid = rbatis::rbdc::Uuid::from_str(&my_uuid_str)?;
    let friend_uuid = rbatis::rbdc::Uuid::from_str(&friend_uuid)?;

    if my_uuid == friend_uuid {
        return Err(anyhow!("Cannot block yourself"));
    }

    let is_exist_friend = is_exist_user_by_uuid(rb, &friend_uuid).await?;
    if !is_exist_friend {
        return Err(anyhow!("Friend user does not exist"));
    }

    let exist = BlackList::select_by_pair(rb, &my_uuid, &friend_uuid).await?;
    if exist.is_some() {
        return Err(anyhow!("Already blocked"));
    }

    let now = get_now_time_stamp_as_millis()?;
    let black_list = BlackList {
        uuid: Some(Uuid::now_v7().to_string().parse()?),
        me_user: Some(my_uuid),
        block_user: Some(friend_uuid),
        created_at: Some(now),
        version: Some(0),
    };
    BlackList::insert(rb, &black_list).await?;

    Ok(crate::utils::http_response::CommonResponseNoDataRef::success_empty())
}

/// 取消拉黑用户
pub async fn unblock_friend_service(
    rb: &RBatis,
    my_uuid: Option<String>,
    friend_uuid: String,
) -> Result<String, anyhow::Error> {
    let my_uuid_str = my_uuid.ok_or(anyhow!("Failed to get current user ID"))?;
    let my_uuid = rbatis::rbdc::Uuid::from_str(&my_uuid_str)?;
    let friend_uuid = rbatis::rbdc::Uuid::from_str(&friend_uuid)?;

    let exist = BlackList::select_by_pair(rb, &my_uuid, &friend_uuid).await?;
    let exist = exist.ok_or(anyhow!("Block relation does not exist"))?;

    let condition = value! {"uuid": exist.uuid.ok_or(anyhow!("Block uuid is None"))?};
    BlackList::delete_by_map(rb, condition).await?;

    Ok(crate::utils::http_response::CommonResponseNoDataRef::success_empty())
}

/// 获取黑名单列表
pub async fn get_black_list_service(
    rb: &RBatis,
    my_uuid: Option<String>,
) -> Result<String, anyhow::Error> {
    let my_uuid_str = my_uuid.ok_or(anyhow!("Failed to get current user ID"))?;
    let my_uuid = rbatis::rbdc::Uuid::from_str(&my_uuid_str)?;

    query_black_list(rb, &my_uuid).await
}
