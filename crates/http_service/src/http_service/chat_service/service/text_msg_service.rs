use anyhow::anyhow;
use common::models::chat_entity::add_read_chat_record::{AddReadChatRecordDTO, CHAT_TYPE_GROUP};
use common::models::chat_entity::chat_message_record::ChatMessageRecord;
use common::models::group_entity::group_message_record::GroupMessageRecord;
use common::models::session_entity::session::SESSION_TYPE_SINGLE;
use common::models::session_entity::user_session::UserSession;
use common::utils::session_uuid::single_session_uuid;
use rbatis::RBatis;
use rbatis::rbdc::Uuid;
use tracing::{info, warn};

use crate::common::dto::base_page_dto::BasePageDTO;
use crate::utils::http_response::{CommonResponseNoDataRef, CommonResponseRef};

/// 计算分页参数: start 取 page_num, size 取 page_size。
/// 历史缺陷: 两参数都读 page_num, page_size 被忽略(任务02 修复 I)。
pub fn page_range(base_page: &BasePageDTO) -> (u32, u32) {
    let start = base_page.page_num.unwrap_or(0);
    let size = base_page.page_size.unwrap_or(10);
    (start, size)
}

/// 获取聊天记录
pub async fn get_chat_by_limit(
    rb: &RBatis,
    uuid: Option<String>,
    friend_uuid: String,
    base_page: BasePageDTO,
) -> Result<String, anyhow::Error> {
    let uuid = uuid.ok_or(anyhow!("账号序列化失败"))?.parse::<Uuid>()?;
    let friend_uuid = friend_uuid.parse::<Uuid>()?;
    let (start, size) = page_range(&base_page);
    let res = ChatMessageRecord::select_chat_by_limit(rb, uuid, friend_uuid, start, size).await?;

    let chat = res.first().ok_or(anyhow!("没有数据"))?;
    let vec = chat.raw.clone();
    let str = String::from_utf8(vec.into_inner())?;
    info!("查询结果: {}", str);
    Ok(CommonResponseRef::<Vec<ChatMessageRecord>>::success_json(&res)?)
}

/// 获取未读消息(临时重写, 修复缺陷A: 全局时间戳 → 按会话游标)。
///
/// 任务05 由 `/session/sync` 正式取代后废弃。
/// 依赖 `user_session` 行存在(上线聚合建立); 无行/无未读返回空。
pub async fn get_unread_chat_record(
    rb: &RBatis,
    uuid: Option<String>,
) -> Result<String, anyhow::Error> {
    let me = uuid.ok_or(anyhow!("账号获取失败"))?.parse::<Uuid>()?;
    let sessions = UserSession::select_by_user(rb, &me).await?;

    let mut unread: Vec<ChatMessageRecord> = Vec::new();
    for s in sessions {
        // 只处理单聊(群聊走 /group/chat/message/unread, 与客户端现状一致)
        if s.session_type != Some(SESSION_TYPE_SINGLE) {
            continue;
        }
        let cursor = s.last_read_id.unwrap_or(0);
        let msgs =
            ChatMessageRecord::select_unread_by_cursor(rb, &s.session_uuid, &me, cursor, 500)
                .await?;
        unread.extend(msgs);
    }

    if unread.is_empty() {
        return Ok(CommonResponseNoDataRef::success_empty());
    }
    Ok(CommonResponseRef::<Vec<ChatMessageRecord>>::success_json(&unread)?)
}

/// 用户已读上报(桥接): 旧 DTO 契约保留, 内部直写 `user_session` 游标。
///
/// Redis 缓冲路径删除(缺陷J: 24h TTL 丢失窗口)。
/// 语义为"读到底" —— 把该会话的 `synced_id` 与 `last_read_id` 都推进到当前最大消息 id
/// (先推 synced 再推 read, 维持 `last_read_id <= synced_id` 不变式)。
pub async fn add_user_chat_read(
    rb: &RBatis,
    uuid: Option<String>,
    chat_message_read: Vec<AddReadChatRecordDTO>,
) -> Result<String, anyhow::Error> {
    let me: Uuid = uuid.ok_or(anyhow!("账号获取失败"))?.parse()?;
    for item in chat_message_read {
        if item.chat_type == Some(CHAT_TYPE_GROUP) {
            // 群聊: nano_id 反查群消息拿 group_uuid(旧路径同款, 频率低可接受)
            let nano_id = item.nano_id.unwrap_or_default();
            let group_uuid = match GroupMessageRecord::select_by_nano_id(rb, &nano_id).await? {
                Some(m) => match m.group_uuid {
                    Some(g) => g,
                    None => continue,
                },
                None => {
                    warn!("[已读桥接] 群消息不存在: {}", nano_id);
                    continue;
                }
            };
            // init 幂等: 新成员一步"建行 + 游标到 max"; 老成员 no-op
            UserSession::init_for_group_join(rb, &me, &group_uuid).await?;
            let max_id = GroupMessageRecord::max_id_by_group(rb, &group_uuid).await?;
            UserSession::update_synced_id(rb, &me, &group_uuid, max_id).await?;
            UserSession::update_last_read_id(rb, &me, &group_uuid, max_id).await?;
        } else {
            // 单聊: 由用户对派生会话(不依赖 nano_id 反查, 零扫描)
            let peer_str = if item.send_user == me {
                item.recv_user.to_string()
            } else {
                item.send_user.to_string()
            };
            let Ok(peer) = peer_str.parse::<Uuid>() else {
                warn!("[已读桥接] 对方 uuid 非法: {}", peer_str);
                continue;
            };
            // rbdc::Uuid 与 uuid::Uuid 是不同类型, 边界转换后派生会话标识
            let session_uuid = single_session_uuid(
                &uuid::Uuid::parse_str(&me.to_string())?,
                &uuid::Uuid::parse_str(&peer.to_string())?,
            )
            .to_string()
            .parse::<Uuid>()?;
            // 行不存在(未聚合)则先建 0 游标行, 保证已读不丢
            UserSession::upsert(
                rb,
                &UserSession {
                    id: None,
                    user_uuid: me.clone(),
                    session_uuid: session_uuid.clone(),
                    session_type: Some(SESSION_TYPE_SINGLE),
                    peer_uuid: Some(peer),
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
            let max_id = ChatMessageRecord::max_id_by_session(rb, &session_uuid).await?;
            UserSession::update_synced_id(rb, &me, &session_uuid, max_id).await?;
            UserSession::update_last_read_id(rb, &me, &session_uuid, max_id).await?;
        }
    }
    Ok(CommonResponseNoDataRef::success_empty())
}
