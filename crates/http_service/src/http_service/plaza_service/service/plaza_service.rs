use std::str::FromStr;

use anyhow::anyhow;
use common::models::plaza_entity::plaza_like::PlazaLike;
use common::models::plaza_entity::plaza_user_info::PlazaUserInfo;
use common::models::plaza_entity::plaza_user_tag::PlazaUserTag;
use common::utils::time::get_now_time_stamp_as_secs;
use rbatis::RBatis;
use rbatis::rbdc::Uuid;
use rbs::value;
use uuid::Uuid as UuidNow;

use crate::http_service::notify_service::service::system_notification::{
    send_plaza_like_msg, send_plaza_match_msg,
};
use crate::http_service::plaza_service::dto::plaza_dto::{
    PlazaCrushToggleDTO, PlazaListQuery, PlazaUpdateProfileDTO,
};
use crate::http_service::plaza_service::vo::plaza_vo::{
    CountRow, PlazaCrushResult, PlazaListVO, PlazaProfileVO, PlazaUserRow, PlazaUserVO,
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
        liked_by_me: row.liked_by_me.map(|v| v > 0).unwrap_or(false),
    }
}

async fn load_tags(rb: &RBatis, uuid: &Uuid) -> Result<Vec<String>, anyhow::Error> {
    let rows = PlazaUserTag::select_by_user(rb, uuid).await?;
    Ok(rows.into_iter().filter_map(|r| r.tag).collect())
}

async fn rows_to_vos(
    rb: &RBatis,
    rows: Vec<PlazaUserRow>,
) -> Result<Vec<PlazaUserVO>, anyhow::Error> {
    let mut list = Vec::with_capacity(rows.len());
    for row in rows {
        let uuid: Uuid = row.uuid.clone().unwrap_or_default().parse()?;
        let tags = load_tags(rb, &uuid).await?;
        list.push(to_vo(row, tags));
    }
    Ok(list)
}

async fn load_username(rb: &RBatis, uuid: &Uuid) -> Result<String, anyhow::Error> {
    let rows: Vec<rbs::Value> = rb
        .exec_decode("select username from basic_user where uuid = ?", vec![value!(uuid.clone())])
        .await?;
    Ok(rows
        .first()
        .and_then(|v| v.as_map())
        .and_then(|m| m["username"].as_str())
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "对方".to_string()))
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
    if let Some(tag) = query.tag {
        let tag = tag.trim().to_string();
        if !tag.is_empty() {
            where_sql.push_str(
                " AND EXISTS (SELECT 1 FROM plaza_user_tag pt WHERE pt.user_uuid = pi.uuid AND pt.tag = ?)",
            );
            args.push(value!(tag));
        }
    }

    let base_sql = format!(
        "SELECT pi.uuid, bu.username, bu.icon, bu.info, ui.gender, ui.age::int as age, \
         ui.address, pi.motto, \
         (SELECT count(*) FROM plaza_like pl WHERE pl.target_uuid = pi.uuid AND pl.user_uuid = ? AND pl.is_del = false) as liked_by_me \
         FROM plaza_user_info pi \
         JOIN basic_user bu ON pi.uuid = bu.uuid \
         LEFT JOIN user_info ui ON pi.uuid = ui.uuid \
         WHERE {where_sql}"
    );

    let count_args = args.clone();
    let count = format!(
        "SELECT count(*) as count FROM plaza_user_info pi LEFT JOIN user_info ui ON pi.uuid = ui.uuid WHERE {where_sql}"
    );
    let count_row: Option<CountRow> = rb
        .exec_decode::<Vec<CountRow>>(&count, count_args)
        .await?
        .into_iter()
        .next();
    let total = count_row.map(|r| r.count).unwrap_or(0) as u32;

    let page_sql = format!("{base_sql} ORDER BY pi.updated_at DESC LIMIT ? OFFSET ?");
    let mut page_args = vec![value!(my_uuid.clone())];
    page_args.extend(args);
    page_args.push(value!(page_size as i64));
    page_args.push(value!(offset));
    let rows: Vec<PlazaUserRow> = rb.exec_decode(&page_sql, page_args).await?;

    let list = rows_to_vos(rb, rows).await?;
    Ok(CommonResponseRef::<PlazaListVO>::success_json(&PlazaListVO { total, list })?)
}

/// 获取我心动的人列表(分页)
pub async fn get_my_liked_list(
    rb: &RBatis,
    my_uuid: Option<String>,
    page_num: u32,
    page_size: u32,
) -> Result<String, anyhow::Error> {
    let me = parse_uuid(my_uuid)?.ok_or_else(|| anyhow!("Failed to get account"))?;
    let page_num = page_num.max(1);
    let page_size = page_size.clamp(1, 50);
    let offset = (page_num as i64 - 1) * page_size as i64;

    let count_sql =
        "SELECT count(*) as count FROM plaza_like pl WHERE pl.user_uuid = ? AND pl.is_del = false";
    let count_row: Option<CountRow> = rb
        .exec_decode::<Vec<CountRow>>(count_sql, vec![value!(me.clone())])
        .await?
        .into_iter()
        .next();
    let total = count_row.map(|r| r.count).unwrap_or(0) as u32;

    let select_sql = "SELECT bu.uuid, bu.username, bu.icon, bu.info, ui.gender, ui.age::int as age, \
        ui.address, pi.motto, \
        (SELECT count(*) FROM plaza_like pl2 WHERE pl2.target_uuid = bu.uuid AND pl2.user_uuid = ? AND pl2.is_del = false) as liked_by_me \
        FROM plaza_like pl JOIN basic_user bu ON pl.target_uuid = bu.uuid \
        LEFT JOIN user_info ui ON bu.uuid = ui.uuid \
        LEFT JOIN plaza_user_info pi ON bu.uuid = pi.uuid \
        WHERE pl.user_uuid = ? AND pl.is_del = false \
        ORDER BY pl.created_at DESC LIMIT ? OFFSET ?";
    let args =
        vec![value!(me.clone()), value!(me.clone()), value!(page_size as i64), value!(offset)];
    let rows: Vec<PlazaUserRow> = rb.exec_decode(select_sql, args).await?;
    let list = rows_to_vos(rb, rows).await?;
    Ok(CommonResponseRef::<PlazaListVO>::success_json(&PlazaListVO { total, list })?)
}

/// 获取互相心动(匹配)列表(分页)
pub async fn get_matched_list(
    rb: &RBatis,
    my_uuid: Option<String>,
    page_num: u32,
    page_size: u32,
) -> Result<String, anyhow::Error> {
    let me = parse_uuid(my_uuid)?.ok_or_else(|| anyhow!("Failed to get account"))?;
    let page_num = page_num.max(1);
    let page_size = page_size.clamp(1, 50);
    let offset = (page_num as i64 - 1) * page_size as i64;

    let count_sql = "SELECT count(*) as count FROM plaza_like ml \
        JOIN plaza_like ol ON ml.target_uuid = ol.user_uuid AND ol.target_uuid = ml.user_uuid \
        WHERE ml.user_uuid = ? AND ml.is_del = false AND ol.is_del = false";
    let count_row: Option<CountRow> = rb
        .exec_decode::<Vec<CountRow>>(count_sql, vec![value!(me.clone())])
        .await?
        .into_iter()
        .next();
    let total = count_row.map(|r| r.count).unwrap_or(0) as u32;

    let select_sql = "SELECT bu.uuid, bu.username, bu.icon, bu.info, ui.gender, ui.age::int as age, \
        ui.address, pi.motto, \
        (SELECT count(*) FROM plaza_like pl2 WHERE pl2.target_uuid = bu.uuid AND pl2.user_uuid = ? AND pl2.is_del = false) as liked_by_me \
        FROM plaza_like ml \
        JOIN plaza_like ol ON ml.target_uuid = ol.user_uuid AND ol.target_uuid = ml.user_uuid \
        JOIN basic_user bu ON ml.target_uuid = bu.uuid \
        LEFT JOIN user_info ui ON bu.uuid = ui.uuid \
        LEFT JOIN plaza_user_info pi ON bu.uuid = pi.uuid \
        WHERE ml.user_uuid = ? AND ml.is_del = false AND ol.is_del = false \
        ORDER BY ml.created_at DESC LIMIT ? OFFSET ?";
    let args =
        vec![value!(me.clone()), value!(me.clone()), value!(page_size as i64), value!(offset)];
    let rows: Vec<PlazaUserRow> = rb.exec_decode(select_sql, args).await?;
    let list = rows_to_vos(rb, rows).await?;
    Ok(CommonResponseRef::<PlazaListVO>::success_json(&PlazaListVO { total, list })?)
}

/// 查询单个广场用户
pub async fn get_plaza_user(
    rb: &RBatis,
    my_uuid: Option<String>,
    uuid_str: String,
) -> Result<String, anyhow::Error> {
    let me = parse_uuid(my_uuid)?.ok_or_else(|| anyhow!("Failed to get account"))?;
    let target = parse_uuid(Some(uuid_str))?.ok_or_else(|| anyhow!("invalid uuid"))?;

    let row: Option<PlazaUserRow> = rb
        .exec_decode::<Vec<PlazaUserRow>>(
            "SELECT pi.uuid, bu.username, bu.icon, bu.info, ui.gender, ui.age::int as age, \
             ui.address, pi.motto, \
             (SELECT count(*) FROM plaza_like pl WHERE pl.target_uuid = pi.uuid AND pl.user_uuid = ? AND pl.is_del = false) as liked_by_me \
             FROM plaza_user_info pi \
             JOIN basic_user bu ON pi.uuid = bu.uuid \
             LEFT JOIN user_info ui ON pi.uuid = ui.uuid \
             WHERE pi.uuid = ? AND pi.allow_discover = true AND pi.status = 0",
            vec![value!(me), value!(target.clone())],
        )
        .await?
        .into_iter()
        .next();
    let Some(row) = row else {
        return Err(anyhow!("User not found or not discoverable"));
    };
    let uuid: Uuid = row.uuid.clone().unwrap_or_default().parse()?;
    let tags = load_tags(rb, &uuid).await?;
    Ok(CommonResponseRef::<PlazaUserVO>::success_json(&to_vo(row, tags))?)
}

/// 心动/取消心动(切换), 返回是否达成互相心动(匹配)
pub async fn switch_crush(
    rb: &RBatis,
    my_uuid: Option<String>,
    dto: PlazaCrushToggleDTO,
) -> Result<String, anyhow::Error> {
    let me = parse_uuid(my_uuid)?.ok_or_else(|| anyhow!("Failed to get account"))?;
    let target = parse_uuid(Some(dto.target_uuid))?.ok_or_else(|| anyhow!("invalid uuid"))?;

    let now = get_now_time_stamp_as_secs()?;
    let existing = PlazaLike::select_by_target_and_user(rb, &target, &me).await?;
    let is_on = match existing {
        Some(mut like) => {
            like.is_del = Some(!like.is_del.unwrap_or(true));
            let id = like.id.clone().ok_or_else(|| anyhow!("like id missing"))?;
            PlazaLike::update_by_map(rb, &like, value! {"id": id}).await?;
            !like.is_del.unwrap_or(true)
        }
        None => {
            let id: Uuid = UuidNow::now_v7().to_string().parse()?;
            let like = PlazaLike {
                id: Some(id),
                target_uuid: Some(target.clone()),
                user_uuid: Some(me.clone()),
                is_del: Some(false),
                created_at: Some(now),
            };
            PlazaLike::insert(rb, &like).await?;
            true
        }
    };

    if is_on {
        let me_name = load_username(rb, &me).await.unwrap_or_else(|_| "对方".to_string());
        let target_name = load_username(rb, &target).await.unwrap_or_else(|_| "对方".to_string());

        // 通知对方: 有人心动了你
        let _ =
            send_plaza_like_msg(rb, target.clone(), format!("{me_name} 在交友广场心动了你"), None)
                .await;

        // 检测是否互相心动(匹配): 对方是否也心动了你
        let mutual = PlazaLike::select_by_target_and_user(rb, &me, &target).await?;
        let matched = mutual.as_ref().map(|l| !l.is_del.unwrap_or(true)).unwrap_or(false);
        if matched {
            let msg = format!("你和 {target_name} 在交友广场互相心动, 可以开始一段新友谊了");
            let _ = send_plaza_match_msg(rb, target.clone(), msg.clone(), None).await;
            let _ = send_plaza_match_msg(rb, me.clone(), msg, None).await;
            return Ok(CommonResponseRef::<PlazaCrushResult>::success_json(&PlazaCrushResult {
                matched: true,
            })?);
        }
    }

    Ok(CommonResponseRef::<PlazaCrushResult>::success_json(&PlazaCrushResult { matched: false })?)
}
