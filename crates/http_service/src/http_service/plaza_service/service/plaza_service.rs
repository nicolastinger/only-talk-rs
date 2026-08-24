use std::str::FromStr;

use anyhow::anyhow;
use common::models::plaza_entity::plaza_user_info::PlazaUserInfo;
use common::models::plaza_entity::plaza_user_tag::PlazaUserTag;
use common::utils::time::get_now_time_stamp_as_secs;
use rbatis::RBatis;
use rbatis::rbdc::Uuid;
use rbs::value;
use uuid::Uuid as UuidNow;

use crate::http_service::plaza_service::dto::plaza_dto::{PlazaListQuery, PlazaUpdateProfileDTO};
use crate::http_service::plaza_service::vo::plaza_vo::{
    CountRow, PlazaListVO, PlazaProfileVO, PlazaUserRow, PlazaUserVO,
};
use crate::utils::http_response::CommonResponseRef;

fn parse_uuid(v: Option<String>) -> Result<Option<Uuid>, anyhow::Error> {
    match v {
        Some(s) => Ok(Some(Uuid::from_str(&s)?)),
        None => Ok(None),
    }
}

fn to_vo(row: PlazaUserRow, tags: Vec<String>) -> PlazaUserVO {
    PlazaUserVO {
        uuid: row.uuid.unwrap_or_default(),
        username: row.username,
        icon: row.icon,
        info: row.info,
        gender: row.gender,
        age: row.age,
        address: row.address,
        motto: row.motto,
        tags,
    }
}

async fn load_tags(rb: &RBatis, uuid: &Uuid) -> Result<Vec<String>, anyhow::Error> {
    let rows = PlazaUserTag::select_by_user(rb, uuid).await?;
    Ok(rows.into_iter().filter_map(|r| r.tag).collect())
}

/// 获取我的广场资料(允许被发现 / 交友宣言 / 标签)
pub async fn get_my_plaza_profile(
    rb: &RBatis,
    my_uuid: Option<String>,
) -> Result<String, anyhow::Error> {
    let uuid = parse_uuid(my_uuid)?.ok_or_else(|| anyhow!("Failed to get account"))?;

    let row = PlazaUserInfo::select_by_uuid(rb, &uuid).await?;
    let tags = load_tags(rb, &uuid).await?;
    let vo = PlazaProfileVO {
        allow_discover: row.as_ref().and_then(|r| r.allow_discover).unwrap_or(false),
        motto: row.as_ref().and_then(|r| r.motto.clone()),
        tags,
    };
    Ok(CommonResponseRef::<PlazaProfileVO>::success_json(&vo)?)
}

/// 更新我的广场资料(允许被发现 / 交友宣言), 懒创建行
pub async fn update_my_plaza_profile(
    rb: &RBatis,
    my_uuid: Option<String>,
    dto: PlazaUpdateProfileDTO,
) -> Result<(), anyhow::Error> {
    let uuid = parse_uuid(my_uuid)?.ok_or_else(|| anyhow!("Failed to get account"))?;

    let tx = rb.acquire_begin().await?;
    let now = get_now_time_stamp_as_secs()?;
    let result = async {
        let existing = PlazaUserInfo::select_by_uuid(rb, &uuid).await?;
        if let Some(mut row) = existing {
            if let Some(ad) = dto.allow_discover {
                row.allow_discover = Some(ad);
            }
            if let Some(m) = dto.motto {
                row.motto = Some(m);
            }
            row.updated_at = Some(now);
            let cond = value! {"uuid": uuid.clone()};
            PlazaUserInfo::update_by_map(rb, &row, cond).await?;
        } else {
            let row = PlazaUserInfo {
                uuid: Some(uuid.clone()),
                allow_discover: Some(dto.allow_discover.unwrap_or(false)),
                motto: dto.motto,
                status: Some(0),
                created_at: Some(now),
                updated_at: Some(now),
            };
            PlazaUserInfo::insert(rb, &row).await?;
        }
        tx.commit().await?;
        Ok(())
    }
    .await;
    if result.is_err() {
        let _ = tx.rollback().await;
    }
    result
}

/// 整组替换我的标签
pub async fn update_my_plaza_tags(
    rb: &RBatis,
    my_uuid: Option<String>,
    tags: Vec<String>,
) -> Result<(), anyhow::Error> {
    let uuid = parse_uuid(my_uuid)?.ok_or_else(|| anyhow!("Failed to get account"))?;

    let mut seen = std::collections::HashSet::new();
    let mut clean = Vec::new();
    for t in tags {
        let t = t.trim().to_string();
        if t.is_empty() || t.len() > 32 {
            continue;
        }
        if !seen.insert(t.clone()) {
            continue;
        }
        clean.push(t);
    }
    if clean.len() > 100 {
        clean.truncate(100);
    }

    let tx = rb.acquire_begin().await?;
    let now = get_now_time_stamp_as_secs()?;
    let result = async {
        PlazaUserTag::delete_by_user(rb, &uuid).await?;
        for (i, tag) in clean.iter().enumerate() {
            let id: Uuid = UuidNow::now_v7().to_string().parse()?;
            let row = PlazaUserTag {
                id: Some(id),
                user_uuid: Some(uuid.clone()),
                tag: Some(tag.clone()),
                sort: Some(i as i32),
                created_at: Some(now),
            };
            PlazaUserTag::insert(rb, &row).await?;
        }
        tx.commit().await?;
        Ok(())
    }
    .await;
    if result.is_err() {
        let _ = tx.rollback().await;
    }
    result
}

/// 分页拉取广场用户(发现流)
pub async fn get_plaza_list(
    rb: &RBatis,
    my_uuid: Option<String>,
    page_num: u32,
    page_size: u32,
    query: PlazaListQuery,
) -> Result<String, anyhow::Error> {
    let my_uuid = parse_uuid(my_uuid)?.ok_or_else(|| anyhow!("Failed to get account"))?;
    let page_num = page_num.max(1);
    let page_size = page_size.clamp(1, 50);
    let offset = (page_num as i64 - 1) * page_size as i64;

    let mut where_sql = String::from("pi.allow_discover = true AND pi.status = 0 AND pi.uuid <> ?");
    let mut args: Vec<rbs::Value> = vec![value!(my_uuid.clone())];
    if let Some(g) = query.gender {
        where_sql.push_str(" AND ui.gender = ?");
        args.push(value!(g as i32));
    }
    if let Some(amin) = query.age_min {
        where_sql.push_str(" AND ui.age::int >= ?");
        args.push(value!(amin as i32));
    }
    if let Some(amax) = query.age_max {
        where_sql.push_str(" AND ui.age::int <= ?");
        args.push(value!(amax as i32));
    }

    let base_sql = format!(
        "SELECT pi.uuid, bu.username, bu.icon, bu.info, ui.gender, ui.age::int as age, \
         ui.address, pi.motto FROM plaza_user_info pi \
         JOIN basic_user bu ON pi.uuid = bu.uuid \
         LEFT JOIN user_info ui ON pi.uuid = ui.uuid \
         WHERE {where_sql}"
    );

    let count_args = args.clone();
    let count = format!(
        "SELECT count(*) as count FROM plaza_user_info pi LEFT JOIN user_info ui ON pi.uuid = ui.uuid WHERE {where_sql}"
    );
    let count_row: Option<CountRow> = rb.query_decode(&count, count_args).await?;
    let total = count_row.map(|r| r.count).unwrap_or(0) as u32;

    let page_sql = format!("{base_sql} ORDER BY pi.updated_at DESC LIMIT ? OFFSET ?");
    args.push(value!(page_size as i64));
    args.push(value!(offset));
    let rows: Vec<PlazaUserRow> = rb.query_decode(&page_sql, args).await?;

    let mut list = Vec::with_capacity(rows.len());
    for row in rows {
        let uuid: Uuid = row.uuid.clone().unwrap_or_default().parse()?;
        let tags = load_tags(rb, &uuid).await?;
        list.push(to_vo(row, tags));
    }
    Ok(CommonResponseRef::<PlazaListVO>::success_json(&PlazaListVO { total, list })?)
}

/// 查询单个广场用户
pub async fn get_plaza_user(
    rb: &RBatis,
    my_uuid: Option<String>,
    uuid_str: String,
) -> Result<String, anyhow::Error> {
    let _my_uuid = parse_uuid(my_uuid)?.ok_or_else(|| anyhow!("Failed to get account"))?;
    let target = parse_uuid(Some(uuid_str))?.ok_or_else(|| anyhow!("invalid uuid"))?;

    let row: Option<PlazaUserRow> = rb
        .query_decode(
            "SELECT pi.uuid, bu.username, bu.icon, bu.info, ui.gender, ui.age::int as age, \
             ui.address, pi.motto FROM plaza_user_info pi \
             JOIN basic_user bu ON pi.uuid = bu.uuid \
             LEFT JOIN user_info ui ON pi.uuid = ui.uuid \
             WHERE pi.uuid = ? AND pi.allow_discover = true AND pi.status = 0",
            vec![value!(target.clone())],
        )
        .await?;
    let Some(row) = row else {
        return Err(anyhow!("User not found or not discoverable"));
    };
    let uuid: Uuid = row.uuid.clone().unwrap_or_default().parse()?;
    let tags = load_tags(rb, &uuid).await?;
    Ok(CommonResponseRef::<PlazaUserVO>::success_json(&to_vo(row, tags))?)
}
