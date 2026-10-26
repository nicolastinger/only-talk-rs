use std::collections::HashSet;
use std::str::FromStr;

use anyhow::anyhow;
use common::models::file_entity::biz_file_link::BizFileLink;
use common::models::file_entity::chat_biz_record::ChatBizRecord;
use common::models::group_entity::group_member::{GroupMember, STATUS_NORMAL};
use common::models::user_entity::friend_link::FriendLink;
use common::utils::time::get_now_time_stamp_as_millis;
use rbatis::{RBatis, rbdc};
use uuid::Uuid;

use crate::http_service::file_service::dto::forward_chat_biz_dto::ForwardChatBizDTO;
use crate::http_service::file_service::vo::forward_chat_biz_vo::ForwardChatBizVO;
use crate::utils::http_response::CommonResponseRef;

/// 创建上传用户聊天文件业务id
pub async fn create_user_chat_biz(
    rb: &RBatis,
    user_id: rbdc::Uuid,
    friend_uuid: rbdc::Uuid,
) -> Result<ChatBizRecord, anyhow::Error> {
    // 1、检测双方是否为好友
    let friend_link = FriendLink::select_by_last_uuid(rb, &user_id, &friend_uuid).await?;
    let is_friend = friend_link.as_ref().map(|link| !link.is_del.unwrap_or(true)).unwrap_or(false);
    if !is_friend {
        return Err(anyhow!("双方不是好友关系，无法发送消息"));
    }

    let now = get_now_time_stamp_as_millis()?;
    let uuid_v4 = Uuid::new_v4();
    let uuid_v4_str = uuid_v4.to_string();
    let biz_id = rbdc::Uuid::from_str(&uuid_v4_str)?;
    let remark = format!("用户聊天上传，用户ID: {}", user_id);

    let chat_biz_record = ChatBizRecord {
        id: None, // ID由数据库自动生成
        uuid: Some(biz_id),
        biz_name: Some("用户聊天上传".to_string()),
        description: Some("用户上传聊天文件的业务记录".to_string()),
        created_by: Some(user_id),
        receiver: Some(friend_uuid),
        created_at: Some(now),
        updated_at: Some(now),
        status: Some(0),                    // 0-正常
        approve_status: Some(1),            // 1-已通过
        biz_type: Some("chat".to_string()), // 业务类型为聊天
        remark: Some(remark),
    };

    // 将业务记录插入数据库
    ChatBizRecord::insert(rb, &chat_biz_record).await?;
    Ok(chat_biz_record)
}

/**
 * 通过业务id获取文件记录信息
 * @param biz_id: 业务id
 */
pub async fn get_chat_file_record_by_biz_id(
    rb: &RBatis,
    biz_id: &str,
) -> Result<ChatBizRecord, anyhow::Error> {
    // 通过业务id查询文件记录信息
    let chat_biz_id = rbdc::Uuid::from_str(biz_id)?;
    let chat_biz_record = ChatBizRecord::select_by_uuid(rb, &chat_biz_id)
        .await?
        .ok_or(anyhow!("未找到对应的文件记录信息"))?;
    Ok(chat_biz_record)
}

/// 创建上传群聊文件业务id（不检查好友关系）
pub async fn create_group_chat_biz(
    rb: &RBatis,
    user_id: rbdc::Uuid,
    group_uuid: rbdc::Uuid,
) -> Result<ChatBizRecord, anyhow::Error> {
    let now = get_now_time_stamp_as_millis()?;
    let uuid_v4 = Uuid::new_v4();
    let uuid_v4_str = uuid_v4.to_string();
    let biz_id = rbdc::Uuid::from_str(&uuid_v4_str)?;
    let remark = format!("群聊上传，用户ID: {}, 群ID: {}", user_id, group_uuid);

    let chat_biz_record = ChatBizRecord {
        id: None,
        uuid: Some(biz_id),
        biz_name: Some("群聊上传".to_string()),
        description: Some("用户上传群聊文件的业务记录".to_string()),
        created_by: Some(user_id),
        receiver: Some(group_uuid),
        created_at: Some(now),
        updated_at: Some(now),
        status: Some(0),
        approve_status: Some(1),
        biz_type: Some("group_chat".to_string()),
        remark: Some(remark),
    };

    ChatBizRecord::insert(rb, &chat_biz_record).await?;
    Ok(chat_biz_record)
}

/// 转发文件前为本次转发目标"重新发牌"：新建 biz 记录并复用原文件关联，不重新上传。
///
/// 每条来源 biz 生成一条新的 `chat_biz_record`：
/// - `created_by` = 当前用户（转发者），`receiver` = 本次转发目标（好友 uuid / 群 uuid）
/// - `biz_type` 跟随目标会话（`chat` / `group_chat`），使下载接口的既有权限判断生效
/// - `biz_file_link` 复制原 biz 的文件关联，指向同一批 S3 对象
///
/// 客户端拿到 `来源 biz -> 新 biz` 映射后，把新 `biz_id` 写进转发消息的 raw，
/// 目标用户即可凭新 biz 通过 `/file/download_link/chat_biz/{biz_id}/1` 下载。
pub async fn forward_chat_biz(
    rb: &RBatis,
    uuid: Option<String>,
    dto: ForwardChatBizDTO,
) -> Result<String, anyhow::Error> {
    let user_id = uuid.ok_or(anyhow!("用户ID为空"))?;
    let user_id = rbdc::Uuid::from_str(&user_id)?;
    let receiver =
        rbdc::Uuid::from_str(dto.receiver.trim()).map_err(|_| anyhow!("接收者ID格式错误"))?;

    // 1. 目标校验：单聊必须仍是好友，群聊必须仍是该群正常成员
    if dto.is_group {
        let member = GroupMember::select_by_group_and_user(rb, &receiver, &user_id).await?;
        let is_member =
            member.map(|m| m.status.unwrap_or(STATUS_NORMAL) == STATUS_NORMAL).unwrap_or(false);
        if !is_member {
            return Err(anyhow!("不在该群聊中，无法转发"));
        }
    } else {
        let friend_link = FriendLink::select_by_last_uuid(rb, &user_id, &receiver).await?;
        let is_friend = friend_link.map(|link| !link.is_del.unwrap_or(true)).unwrap_or(false);
        if !is_friend {
            return Err(anyhow!("双方不是好友关系，无法转发"));
        }
    }

    let now = get_now_time_stamp_as_millis()?;
    let (biz_type, biz_name, description) = if dto.is_group {
        ("group_chat", "群聊转发文件", "群聊转发文件的业务记录")
    } else {
        ("chat", "用户聊天转发文件", "转发聊天文件的业务记录")
    };

    let mut result: Vec<ForwardChatBizVO> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for source_biz_id in dto.biz_ids {
        let source_biz_id = source_biz_id.trim().to_string();
        if source_biz_id.is_empty() || !seen.insert(source_biz_id.clone()) {
            continue;
        }
        let source_uuid = rbdc::Uuid::from_str(&source_biz_id)
            .map_err(|_| anyhow!("业务ID格式错误: {}", source_biz_id))?;
        let source = get_chat_file_record_by_biz_id(rb, &source_biz_id).await?;

        // 2. 只能转发自己可访问的文件（与下载接口的权限判断保持一致）
        let source_is_group = source.biz_type.as_deref() == Some("group_chat");
        if !source_is_group
            && source.created_by.as_ref() != Some(&user_id)
            && source.receiver.as_ref() != Some(&user_id)
        {
            return Err(anyhow!("无权限访问"));
        }

        // 3. 先确认原 biz 有可复用的文件关联，避免生成空业务记录
        let links = BizFileLink::select_by_biz(rb, &source_uuid).await?;
        let links: Vec<BizFileLink> = links
            .into_iter()
            .filter(|l| !l.is_del.unwrap_or(false))
            .filter(|l| l.file_id.is_some() || l.origin_file_id.is_some())
            .collect();
        if links.is_empty() {
            return Err(anyhow!("文件关联为空，无法转发: {}", source_biz_id));
        }

        // 4. 新业务记录：接收者写成本次转发目标
        let new_biz_id = rbdc::Uuid::from_str(&Uuid::new_v4().to_string())?;
        let record = ChatBizRecord {
            id: None,
            uuid: Some(new_biz_id.clone()),
            biz_name: Some(biz_name.to_string()),
            description: Some(description.to_string()),
            created_by: Some(user_id.clone()),
            receiver: Some(receiver.clone()),
            created_at: Some(now),
            updated_at: Some(now),
            status: Some(0),
            approve_status: Some(1),
            biz_type: Some(biz_type.to_string()),
            remark: Some(format!("转发自业务 {}，用户ID: {}", source_biz_id, user_id)),
        };
        ChatBizRecord::insert(rb, &record).await?;

        // 5. 复用原文件关联（同一批 file_id/origin_file_id，不重新上传）
        for link in links {
            BizFileLink::insert(
                rb,
                &BizFileLink {
                    id: None,
                    biz_id: Some(new_biz_id.clone()),
                    origin_file_id: link.origin_file_id,
                    file_id: link.file_id,
                    is_del: Some(false),
                },
            )
            .await?;
        }

        result.push(ForwardChatBizVO { source_biz_id, biz_id: new_biz_id.to_string() });
    }

    if result.is_empty() {
        return Err(anyhow!("没有需要签发的文件"));
    }
    Ok(CommonResponseRef::<Vec<ForwardChatBizVO>>::success_json(&result)?)
}
