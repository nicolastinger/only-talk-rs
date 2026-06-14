use std::str::FromStr;

use anyhow::anyhow;
use common::models::file_entity::chat_biz_record::ChatBizRecord;
use common::models::user_entity::friend_link::FriendLink;
use common::utils::time::get_now_time_stamp_as_millis;
use rbatis::{RBatis, rbdc};
use uuid::Uuid;

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
