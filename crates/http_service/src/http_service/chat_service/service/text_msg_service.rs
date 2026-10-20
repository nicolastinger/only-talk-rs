use anyhow::anyhow;
use common::models::chat_entity::chat_message_record::ChatMessageRecord;
use common::utils::session_uuid::single_session_uuid;
use rbatis::RBatis;
use rbatis::rbdc::Uuid;
use tracing::info;

use crate::common::dto::base_page_dto::BasePageDTO;
use crate::utils::http_response::CommonResponseRef;

/// 计算分页参数: start 取 page_num, size 取 page_size。
/// 历史缺陷: 两参数都读 page_num, page_size 被忽略(任务02 修复 I)。
pub fn page_range(base_page: &BasePageDTO) -> (u32, u32) {
    let start = base_page.page_num.unwrap_or(0);
    let size = base_page.page_size.unwrap_or(10);
    (start, size)
}

/// 获取聊天记录。
///
/// 任务08: Body 可选 `session_uuid` 直达会话(分区剪枝 + 索引扫描);
/// 缺省时由 path 的 `friend_uuid` 派生(零契约破坏, 旧客户端不受影响)。
pub async fn get_chat_by_limit(
    rb: &RBatis,
    uuid: Option<String>,
    friend_uuid: String,
    base_page: BasePageDTO,
) -> Result<String, anyhow::Error> {
    let me: Uuid = uuid.ok_or(anyhow!("账号序列化失败"))?.parse()?;
    let (start, size) = page_range(&base_page);

    let session_uuid: Uuid = match base_page.session_uuid.as_deref() {
        Some(s) => s.parse()?,
        None => {
            let friend: Uuid = friend_uuid.parse()?;
            single_session_uuid(
                &uuid::Uuid::parse_str(&me.to_string())?,
                &uuid::Uuid::parse_str(&friend.to_string())?,
            )
            .to_string()
            .parse()?
        }
    };

    let res = ChatMessageRecord::select_by_session_paged(rb, &session_uuid, start, size).await?;

    let chat = res.first().ok_or(anyhow!("没有数据"))?;
    let vec = chat.raw.clone();
    let str = String::from_utf8(vec.into_inner())?;
    info!("查询结果: {}", str);
    Ok(CommonResponseRef::<Vec<ChatMessageRecord>>::success_json(&res)?)
}
