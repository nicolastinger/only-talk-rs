use std::str::FromStr;

use anyhow::anyhow;
use common::models::announcement_entity::announcement::Announcement;
use common::models::announcement_entity::announcement_read::AnnouncementRead;
use common::utils::time::get_now_time_stamp_as_secs;
use rbatis::RBatis;
use rbatis::rbdc::Uuid;
use rbs::value;
use uuid::Uuid as UuidNow;

use crate::http_service::announcement_service::vo::announcement_vo::{
    AnnouncementListVO, AnnouncementReadUserListVO, AnnouncementReadUserRow,
    AnnouncementReadUserVO, AnnouncementRow, AnnouncementVO, CountRow,
};
use crate::utils::http_response::CommonResponseRef;

fn parse_uuid(v: Option<String>) -> Result<Option<Uuid>, anyhow::Error> {
    match v {
        Some(s) => Ok(Some(Uuid::from_str(&s)?)),
        None => Ok(None),
    }
}

fn to_vo(row: AnnouncementRow) -> AnnouncementVO {
    AnnouncementVO {
        uuid: row.uuid.unwrap_or_default(),
        title: row.title.unwrap_or_default(),
        content: row.content.unwrap_or_default(),
        content_type: row.content_type.unwrap_or(0),
        start_at: row.start_at.unwrap_or(0),
        end_at: row.end_at.unwrap_or(0),
        created_at: row.created_at.unwrap_or(0),
        is_read: row.is_read.map(|v| v > 0).unwrap_or(false),
        read_count: row.read_count.unwrap_or(0),
    }
}

/// 分页拉取当前时间窗口内所有启用的公告(供前端轮播)
pub async fn get_announcement_list(
    rb: &RBatis,
    my_uuid: Option<String>,
    page_num: u32,
    page_size: u32,
) -> Result<String, anyhow::Error> {
    let me = parse_uuid(my_uuid)?.ok_or_else(|| anyhow!("Failed to get account"))?;
    let page_num = page_num.max(1);
    let page_size = page_size.clamp(1, 50);
    let offset = (page_num as i64 - 1) * page_size as i64;
    let now = get_now_time_stamp_as_secs()?;

    let count_sql = "SELECT count(*) as count FROM announcement a WHERE a.is_del = false AND a.is_active = true AND a.start_at <= ? AND a.end_at >= ?";
    let count_row: Option<CountRow> =
        rb.exec_decode::<Vec<CountRow>>(count_sql, vec![value!(now), value!(now)])
            .await?
            .into_iter()
            .next();
    let total = count_row.map(|r| r.count).unwrap_or(0) as u32;

    let select_sql = "SELECT a.uuid, a.title, a.content, a.content_type::int as content_type, \
        a.start_at, a.end_at, a.created_at, \
        (SELECT count(*) FROM announcement_read ar WHERE ar.announcement_uuid = a.uuid) as read_count, \
        (SELECT count(*) FROM announcement_read ar2 WHERE ar2.announcement_uuid = a.uuid AND ar2.user_uuid = ?) as is_read \
        FROM announcement a WHERE a.is_del = false AND a.is_active = true AND a.start_at <= ? AND a.end_at >= ? \
        ORDER BY a.sort_order ASC, a.created_at DESC LIMIT ? OFFSET ?";
    let args = vec![
        value!(me.clone()),
        value!(now),
        value!(now),
        value!(page_size as i64),
        value!(offset),
    ];
    let rows: Vec<AnnouncementRow> = rb.exec_decode(select_sql, args).await?;
    let list = rows.into_iter().map(to_vo).collect();
    Ok(CommonResponseRef::<AnnouncementListVO>::success_json(&AnnouncementListVO { total, list })?)
}

/// 公告详情(含当前用户已读状态)
pub async fn get_announcement_detail(
    rb: &RBatis,
    my_uuid: Option<String>,
    announcement_uuid_str: String,
) -> Result<String, anyhow::Error> {
    let me = parse_uuid(my_uuid)?.ok_or_else(|| anyhow!("Failed to get account"))?;
    let announcement_uuid =
        parse_uuid(Some(announcement_uuid_str))?.ok_or_else(|| anyhow!("invalid uuid"))?;

    let select_sql = "SELECT a.uuid, a.title, a.content, a.content_type::int as content_type, \
        a.start_at, a.end_at, a.created_at, \
        (SELECT count(*) FROM announcement_read ar WHERE ar.announcement_uuid = a.uuid) as read_count, \
        (SELECT count(*) FROM announcement_read ar2 WHERE ar2.announcement_uuid = a.uuid AND ar2.user_uuid = ?) as is_read \
        FROM announcement a WHERE a.uuid = ? AND a.is_del = false AND a.is_active = true";
    let args = vec![value!(me.clone()), value!(announcement_uuid.clone())];
    let row: Option<AnnouncementRow> = rb
        .exec_decode::<Vec<AnnouncementRow>>(select_sql, args)
        .await?
        .into_iter()
        .next();
    let Some(row) = row else {
        return Err(anyhow!("公告不存在"));
    };
    Ok(CommonResponseRef::<AnnouncementVO>::success_json(&to_vo(row))?)
}

/// 标记已读(幂等，重复调用不报错)
pub async fn mark_announcement_read(
    rb: &RBatis,
    my_uuid: Option<String>,
    announcement_uuid_str: String,
) -> Result<String, anyhow::Error> {
    let me = parse_uuid(my_uuid.clone())?.ok_or_else(|| anyhow!("Failed to get account"))?;
    let announcement_uuid =
        parse_uuid(Some(announcement_uuid_str))?.ok_or_else(|| anyhow!("invalid uuid"))?;

    if Announcement::select_by_uuid(rb, &announcement_uuid).await?.is_none() {
        return Err(anyhow!("公告不存在"));
    }

    let already =
        AnnouncementRead::select_by_announcement_and_user(rb, &announcement_uuid, &me).await?;
    if already.is_none() {
        let now = get_now_time_stamp_as_secs()?;
        let id: Uuid = UuidNow::now_v7().to_string().parse()?;
        let read = AnnouncementRead {
            id: Some(id),
            announcement_uuid: Some(announcement_uuid.clone()),
            user_uuid: Some(me.clone()),
            created_at: Some(now),
        };
        AnnouncementRead::insert(rb, &read).await?;
    }

    get_announcement_detail(rb, my_uuid, announcement_uuid.to_string()).await
}

/// 分页拉取已读用户
pub async fn get_announcement_read_users(
    rb: &RBatis,
    my_uuid: Option<String>,
    announcement_uuid_str: String,
    page_num: u32,
    page_size: u32,
) -> Result<String, anyhow::Error> {
    let _me = parse_uuid(my_uuid)?.ok_or_else(|| anyhow!("Failed to get account"))?;
    let announcement_uuid =
        parse_uuid(Some(announcement_uuid_str))?.ok_or_else(|| anyhow!("invalid uuid"))?;
    let page_num = page_num.max(1);
    let page_size = page_size.clamp(1, 50);
    let offset = (page_num as i64 - 1) * page_size as i64;

    let count_sql =
        "SELECT count(*) as count FROM announcement_read ar WHERE ar.announcement_uuid = ?";
    let count_row: Option<CountRow> =
        rb.exec_decode::<Vec<CountRow>>(count_sql, vec![value!(announcement_uuid.clone())])
            .await?
            .into_iter()
            .next();
    let total = count_row.map(|r| r.count).unwrap_or(0) as u32;

    let select_sql = "SELECT bu.uuid, bu.username, bu.icon, ar.created_at \
        FROM announcement_read ar JOIN basic_user bu ON ar.user_uuid = bu.uuid \
        WHERE ar.announcement_uuid = ? ORDER BY ar.created_at DESC LIMIT ? OFFSET ?";
    let rows: Vec<AnnouncementReadUserRow> = rb
        .exec_decode(
            select_sql,
            vec![value!(announcement_uuid.clone()), value!(page_size as i64), value!(offset)],
        )
        .await?;

    let list = rows
        .into_iter()
        .map(|r| AnnouncementReadUserVO {
            uuid: r.uuid.unwrap_or_default(),
            username: r.username,
            icon: r.icon,
            created_at: r.created_at.unwrap_or(0),
        })
        .collect();
    Ok(CommonResponseRef::<AnnouncementReadUserListVO>::success_json(
        &AnnouncementReadUserListVO { total, list },
    )?)
}
