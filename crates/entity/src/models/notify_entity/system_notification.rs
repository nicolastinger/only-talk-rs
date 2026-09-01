use rbatis::rbdc::Uuid;
use rbatis::{RBatis, crud, impl_select};
use rbs::value;
use serde::{Deserialize, Serialize};

/// 系统通知
#[derive(Debug, Serialize, Deserialize)]
pub struct SystemNotification {
    pub id: Option<Uuid>,
    /// 通知标题
    pub title: Option<String>,
    /// 详细内容
    pub content: Option<String>,
    /// 创建时间
    pub created_at: Option<i64>,
    /// 内容类型，0-纯文本，1-json, 2-xml
    pub content_type: Option<u16>,
    /// 接收人
    pub user_id: Option<Uuid>,
    /// 是否已读
    pub is_read: Option<bool>,
    /// 业务ID
    pub biz_id: Option<String>,

    /// 第一层级，用于定位功能大类
    pub level1: Option<i32>,
    /// 第二层级，用于定位子功能模块
    pub level2: Option<i32>,
    /// 第三层级，用于定位具体功能组
    pub level3: Option<i32>,
    /// 第四层级，用于定位详细功能项
    pub level4: Option<i32>,
    /// 未读数量
    pub unread_count: Option<i32>,
    /// 通知优先级
    pub priority: Option<i32>,
}

crud!(SystemNotification {});

impl_select!(SystemNotification{select_all_by_uid(user_id:&Uuid, is_read: Option<bool>) => "`where user_id = #{user_id} and (#{is_read} is null or is_read = #{is_read})`"});

impl_select!(SystemNotification{select_unread_in_window(user_id:&Uuid, is_read: bool, since: i64) => "`where user_id = #{user_id} and is_read = #{is_read} and created_at >= #{since} order by created_at desc`"});

/// 批量标记已读（幂等）：按 id 文本匹配，仅更新当前用户的通知
pub async fn mark_read_by_ids(
    rbatis: &RBatis,
    user_id: &Uuid,
    ids: &Vec<String>,
) -> Result<u64, rbatis::Error> {
    if ids.is_empty() {
        return Ok(0);
    }
    let placeholders: Vec<String> = (1..=ids.len()).map(|i| format!("${}", i + 1)).collect();
    let sql = format!(
        "UPDATE system_notification SET is_read = true WHERE user_id = $1::uuid AND id::text IN ({})",
        placeholders.join(", ")
    );
    let mut args = vec![value!(user_id.clone())];
    for id in ids {
        args.push(value!(id));
    }
    let result = rbatis.exec(&sql, args).await?;
    Ok(result.rows_affected as u64)
}
