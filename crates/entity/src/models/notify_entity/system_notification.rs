use rbatis::executor::Executor;
use rbatis::rbdc::Uuid;
use rbatis::{RBatis, crud};
use rbs::value;
use serde::{Deserialize, Serialize};

/// 系统通知
#[derive(Clone, Debug, Serialize, Deserialize)]
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

/// 按 (level1, level2) 分组的未读数(供通知中心分类角标)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NotificationLevelCount {
    pub level1: Option<i32>,
    pub level2: Option<i32>,
    pub count: i64,
}

/// 通知中心列表查询参数(收敛 select_page 过多入参)
#[derive(Clone, Debug, Serialize)]
pub struct NotificationPageQuery {
    pub user_id: Uuid,
    /// 窗口起点(created_at >= since)
    pub since: i64,
    /// 已读过滤: None=全部
    pub is_read: Option<bool>,
    /// 分类过滤: 一级功能大类
    pub level1: Option<i32>,
    /// 分类过滤: 二级子模块
    pub level2: Option<i32>,
    /// keyset 游标时间(首页为 None)
    pub cursor_at: Option<i64>,
    /// keyset 游标 id(首页为 None)
    pub cursor_id: Option<Uuid>,
    /// 每页条数(调用方传 limit+1 探测 has_more)
    pub size: u32,
}

impl SystemNotification {
    #[rbatis::py_sql(
        "select * from system_notification where user_id = #{user_id} and is_read = #{is_read} and created_at >= #{since} order by created_at desc"
    )]
    async fn select_unread_in_window(
        rb: &dyn Executor,
        user_id: &Uuid,
        is_read: bool,
        since: i64,
    ) -> Vec<SystemNotification> {
    }

    /// 通知中心列表(keyset 分页 + 分类过滤 + 窗口过滤)。
    ///
    /// 排序键 `(created_at desc, id desc)` 与游标列同源; `size` 由调用方传 `limit+1`
    /// 探测 `has_more`。游标占位符显式转型,避免首页 NULL 参数导致语句缓存类型冲突
    /// (任务05 缺陷 T 同源)。
    #[rbatis::py_sql(
        "select * from system_notification
         where user_id = #{q.user_id}
           and created_at >= #{q.since}
           and (#{q.is_read} is null or is_read = #{q.is_read})
           and (#{q.level1} is null or level1 = #{q.level1})
           and (#{q.level2} is null or level2 = #{q.level2})
           and (#{q.cursor_at}::int8 is null
                or (created_at, id) < (#{q.cursor_at}::int8, #{q.cursor_id}::uuid))
         order by created_at desc, id desc
         limit #{q.size}"
    )]
    async fn select_page_inner(
        rb: &dyn Executor,
        q: NotificationPageQuery,
    ) -> Vec<SystemNotification> {
    }

    pub async fn select_page(
        rb: &dyn Executor,
        q: NotificationPageQuery,
    ) -> Result<Vec<SystemNotification>, rbatis::Error> {
        Self::select_page_inner(rb, q).await
    }

    /// 统计未读总数(窗口内)
    #[rbatis::py_sql(
        "select count(*) as count from system_notification where user_id = #{user_id} and is_read = false and created_at >= #{since}"
    )]
    async fn count_unread_total_inner(
        rb: &dyn Executor,
        user_id: &Uuid,
        since: i64,
    ) -> Vec<NotificationLevelCount> {
    }

    pub async fn count_unread_total(
        rb: &dyn Executor,
        user_id: &Uuid,
        since: i64,
    ) -> Result<i64, rbatis::Error> {
        Ok(Self::count_unread_total_inner(rb, user_id, since)
            .await?
            .into_iter()
            .next()
            .map(|r| r.count)
            .unwrap_or(0))
    }

    /// 按 (level1, level2) 分组统计未读(窗口内),供分类角标
    #[rbatis::py_sql(
        "select level1, level2, count(*) as count from system_notification
         where user_id = #{user_id} and is_read = false and created_at >= #{since}
         group by level1, level2"
    )]
    async fn count_unread_grouped_inner(
        rb: &dyn Executor,
        user_id: &Uuid,
        since: i64,
    ) -> Vec<NotificationLevelCount> {
    }

    pub async fn count_unread_grouped(
        rb: &dyn Executor,
        user_id: &Uuid,
        since: i64,
    ) -> Result<Vec<NotificationLevelCount>, rbatis::Error> {
        Self::count_unread_grouped_inner(rb, user_id, since).await
    }

    /// 同源未读聚合: 若存在相同 `(user_id, level1, level2, level3, biz_id)` 的未读通知,
    /// 累加 `unread_count` 并刷新内容/时间(如多次点赞合并计数); 否则插入新行。
    ///
    /// 返回最终落库的通知行(聚合时为新行或更新后的旧行)。
    pub async fn upsert_unread_aggregate(
        rb: &RBatis,
        notification: &SystemNotification,
    ) -> Result<SystemNotification, rbatis::Error> {
        let user_id = notification.user_id.clone();
        let since = notification.created_at.unwrap_or(0);

        // 查找同源未读(窗口内取最近一条)
        let existing = rb
            .exec_decode::<Vec<SystemNotification>>(
                "select * from system_notification
                 where user_id = $1
                   and is_read = false
                   and level1 is not distinct from $2
                   and level2 is not distinct from $3
                   and level3 is not distinct from $4
                   and biz_id is not distinct from $5
                   and created_at >= $6
                 order by created_at desc
                 limit 1",
                vec![
                    value!(user_id),
                    value!(&notification.level1),
                    value!(&notification.level2),
                    value!(&notification.level3),
                    value!(&notification.biz_id),
                    value!(since),
                ],
            )
            .await?
            .into_iter()
            .next();

        if let Some(mut found) = existing {
            let new_count = found.unread_count.unwrap_or(1).saturating_add(1);
            let id = found.id.clone();
            rb.exec(
                "update system_notification
                 set unread_count = $1, content = $2, created_at = $3
                 where id = $4",
                vec![
                    value!(new_count),
                    value!(&notification.content),
                    value!(&notification.created_at),
                    value!(&id),
                ],
            )
            .await?;
            found.unread_count = Some(new_count);
            found.content = notification.content.clone();
            found.created_at = notification.created_at;
            return Ok(found);
        }

        SystemNotification::insert(rb, notification).await?;
        Ok(notification.clone())
    }
}

/// 批量标记已读（幂等）：按 id 文本匹配，仅更新当前用户的通知
pub async fn mark_read_by_ids(
    rbatis: &RBatis,
    user_id: &Uuid,
    ids: &[String],
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

/// 标记用户全部未读通知为已读(可选按分类过滤),返回影响行数
pub async fn mark_all_read(
    rbatis: &RBatis,
    user_id: &Uuid,
    level1: Option<i32>,
    level2: Option<i32>,
) -> Result<u64, rbatis::Error> {
    let result = rbatis
        .exec(
            "UPDATE system_notification SET is_read = true
             WHERE user_id = $1::uuid AND is_read = false
               AND ($2::int4 IS NULL OR level1 = $2)
               AND ($3::int4 IS NULL OR level2 = $3)",
            vec![value!(user_id.clone()), value!(level1), value!(level2)],
        )
        .await?;
    Ok(result.rows_affected as u64)
}
