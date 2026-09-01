use std::str::FromStr;

use anyhow::anyhow;
use common::models::file_entity::biz_file_link::BizFileLink;
use common::models::file_entity::biz_record::BizRecord;
use common::models::moment_entity::moment::Moment;
use common::models::moment_entity::moment_comment::MomentComment;
use common::models::moment_entity::moment_like::MomentLike;
use common::models::moment_entity::user_follow::UserFollow;
use common::utils::time::get_now_time_stamp_as_secs;
use rbatis::RBatis;
use rbatis::rbdc::Uuid;
use rbs::value;
use uuid::Uuid as UuidNow;

use crate::http_service::moment_service::dto::moment_dto::{
    AddCommentDTO, CreateMomentDTO, DeleteMomentDTO, FollowToggleDTO, LikeToggleDTO,
};
use crate::http_service::moment_service::vo::moment_vo::{
    CountRow, MomentCommentListVO, MomentCommentRow, MomentCommentVO, MomentLikerListVO,
    MomentLikerRow, MomentLikerVO, MomentListVO, MomentRow, MomentVO,
};
use crate::utils::http_response::CommonResponseRef;

fn parse_uuid(v: Option<String>) -> Result<Option<Uuid>, anyhow::Error> {
    match v {
        Some(s) => Ok(Some(Uuid::from_str(&s)?)),
        None => Ok(None),
    }
}

fn to_vo(row: MomentRow) -> MomentVO {
    MomentVO {
        uuid: row.uuid.unwrap_or_default(),
        author_uuid: row.author_uuid.unwrap_or_default(),
        username: row.username,
        icon: row.icon,
        content: row.content.unwrap_or_default(),
        visibility: row.visibility.unwrap_or(0),
        image_count: row.image_count.unwrap_or(0),
        like_count: row.like_count.unwrap_or(0),
        comment_count: row.comment_count.unwrap_or(0),
        liked_by_me: row.liked_by_me.map(|v| v > 0).unwrap_or(false),
        followed_by_me: row.followed_by_me.map(|v| v > 0).unwrap_or(false),
        created_at: row.created_at.unwrap_or(0),
        updated_at: row.updated_at.unwrap_or(0),
    }
}

fn to_comment_vo(row: MomentCommentRow) -> MomentCommentVO {
    MomentCommentVO {
        id: row.id.unwrap_or_default(),
        moment_uuid: row.moment_uuid.unwrap_or_default(),
        author_uuid: row.author_uuid.unwrap_or_default(),
        username: row.username,
        icon: row.icon,
        content: row.content.unwrap_or_default(),
        created_at: row.created_at.unwrap_or(0),
    }
}

fn to_liker_vo(row: MomentLikerRow) -> MomentLikerVO {
    MomentLikerVO {
        uuid: row.uuid.unwrap_or_default(),
        username: row.username,
        icon: row.icon,
        created_at: row.created_at.unwrap_or(0),
    }
}

/// 校验动态存在且当前用户可见(公开或作者本人)
async fn ensure_moment_visible(
    rb: &RBatis,
    moment_uuid: &Uuid,
    me: &Uuid,
) -> Result<(), anyhow::Error> {
    let moment = Moment::select_by_uuid(rb, moment_uuid).await?;
    match moment {
        Some(m) => {
            if m.is_del.unwrap_or(false) {
                return Err(anyhow!("动态不存在"));
            }
            let visibility = m.visibility.unwrap_or(0);
            if visibility == 1 && m.author_uuid.as_ref() != Some(me) {
                return Err(anyhow!("无权访问该动态"));
            }
            Ok(())
        }
        None => Err(anyhow!("动态不存在")),
    }
}

/// 发布动态
pub async fn create_moment(
    rb: &RBatis,
    my_uuid: Option<String>,
    dto: CreateMomentDTO,
) -> Result<String, anyhow::Error> {
    let me = parse_uuid(my_uuid)?.ok_or_else(|| anyhow!("Failed to get account"))?;
    let content = dto.content.trim().to_string();
    if content.is_empty() {
        return Err(anyhow!("内容不能为空"));
    }
    let content =
        if content.chars().count() > 2000 { content.chars().take(2000).collect() } else { content };
    if dto.file_ids.len() > 9 {
        return Err(anyhow!("最多上传9张图片"));
    }
    let visibility = if dto.visibility == 1 { 1i16 } else { 0i16 };
    let moment_uuid: Uuid = UuidNow::now_v7().to_string().parse()?;

    let tx = rb.acquire_begin().await?;
    let result: Result<(), anyhow::Error> = async {
        let now = get_now_time_stamp_as_secs()?;

        let moment = Moment {
            uuid: Some(moment_uuid.clone()),
            author_uuid: Some(me.clone()),
            content: Some(content.clone()),
            visibility: Some(visibility),
            is_del: Some(false),
            created_at: Some(now),
            updated_at: Some(now),
        };
        Moment::insert(rb, &moment).await?;

        if !dto.file_ids.is_empty() {
            let biz_record = BizRecord {
                id: None,
                uuid: Some(moment_uuid.clone()),
                biz_name: Some("动态".to_string()),
                description: Some("动态广场图片业务".to_string()),
                created_by: Some(me.clone()),
                created_at: Some(now),
                updated_at: Some(now),
                status: Some(0),
                approve_status: Some(1),
                biz_type: Some("moment".to_string()),
                remark: Some(format!("动态{}图片，作者: {}", moment_uuid, me)),
            };
            BizRecord::insert(rb, &biz_record).await?;

            for fid in &dto.file_ids {
                let file_id = Uuid::from_str(fid)?;
                let link = BizFileLink {
                    id: None,
                    biz_id: Some(moment_uuid.clone()),
                    origin_file_id: None,
                    file_id: Some(file_id),
                    is_del: Some(false),
                };
                BizFileLink::insert(rb, &link).await?;
            }
        }

        tx.commit().await?;
        Ok(())
    }
    .await;
    if result.is_err() {
        let _ = tx.rollback().await;
    }
    result?;

    get_moment_detail(rb, Some(me.to_string()), moment_uuid.to_string()).await
}

/// 构建动态流筛选: 返回 (WHERE 片段, 参数)
/// feed: plaza-广场, following-关注, mine-我的; None 时保留旧逻辑(公开+自己)
fn build_moment_filter(
    me: &Uuid,
    feed: Option<&str>,
    author: Option<&Uuid>,
) -> (String, Vec<rbs::Value>) {
    let mut where_sql = String::from("m.is_del = false AND (");
    let mut args = Vec::new();
    match feed.unwrap_or("") {
        "mine" => {
            where_sql.push_str("m.author_uuid = ?");
            args.push(value!(me.clone()));
        }
        "following" => {
            where_sql.push_str(
                "m.visibility = 0 AND m.author_uuid IN (SELECT target_user_uuid FROM user_follow WHERE follow_user_uuid = ? AND is_del = false)",
            );
            args.push(value!(me.clone()));
        }
        "plaza" => {
            where_sql.push_str("m.visibility = 0");
        }
        _ => {
            where_sql.push_str("m.visibility = 0 OR m.author_uuid = ?");
            args.push(value!(me.clone()));
        }
    }
    where_sql.push(')');
    if let Some(a) = author {
        where_sql.push_str(" AND m.author_uuid = ?");
        args.push(value!(a.clone()));
    }
    (where_sql, args)
}

/// 分页拉取动态流(按 feed: plaza/following/mine; 缺省=公开+自己)
pub async fn get_moment_list(
    rb: &RBatis,
    my_uuid: Option<String>,
    page_num: u32,
    page_size: u32,
    author_uuid: Option<String>,
    feed: Option<String>,
) -> Result<String, anyhow::Error> {
    let me = parse_uuid(my_uuid)?.ok_or_else(|| anyhow!("Failed to get account"))?;
    let page_num = page_num.max(1);
    let page_size = page_size.clamp(1, 50);
    let offset = (page_num as i64 - 1) * page_size as i64;

    let author = author_uuid.map(|s| parse_uuid(Some(s))).transpose()?.flatten();

    let (where_sql, where_args) = build_moment_filter(&me, feed.as_deref(), author.as_ref());

    let count_sql = format!("SELECT count(*) as count FROM moment m WHERE {where_sql}");
    let count_row: Option<CountRow> = rb.exec_decode(&count_sql, where_args.clone()).await?;
    let total = count_row.map(|r| r.count).unwrap_or(0) as u32;

    let select_sql = format!(
        "SELECT m.uuid, m.author_uuid, m.content, m.visibility::int as visibility, \
        m.created_at, m.updated_at, bu.username, bu.icon, \
        (SELECT count(*) FROM biz_file_link bf WHERE bf.biz_id = m.uuid) as image_count, \
        (SELECT count(*) FROM moment_like ml WHERE ml.moment_uuid = m.uuid AND ml.is_del = false) as like_count, \
        (SELECT count(*) FROM moment_comment mc WHERE mc.moment_uuid = m.uuid AND mc.is_del = false) as comment_count, \
        (SELECT count(*) FROM moment_like ml2 WHERE ml2.moment_uuid = m.uuid AND ml2.user_uuid = ? AND ml2.is_del = false) as liked_by_me, \
        (SELECT count(*) FROM user_follow uf WHERE uf.follow_user_uuid = ? AND uf.target_user_uuid = m.author_uuid AND uf.is_del = false) as followed_by_me \
        FROM moment m JOIN basic_user bu ON m.author_uuid = bu.uuid \
        WHERE {where_sql} ORDER BY m.created_at DESC LIMIT ? OFFSET ?"
    );
    let mut args = vec![value!(me.clone()), value!(me.clone())];
    args.extend(where_args);
    args.push(value!(page_size as i64));
    args.push(value!(offset));
    let rows: Vec<MomentRow> = rb.exec_decode(&select_sql, args).await?;

    let list = rows.into_iter().map(to_vo).collect();
    Ok(CommonResponseRef::<MomentListVO>::success_json(&MomentListVO { total, list })?)
}

/// 查询单条动态
pub async fn get_moment_detail(
    rb: &RBatis,
    my_uuid: Option<String>,
    moment_uuid_str: String,
) -> Result<String, anyhow::Error> {
    let me = parse_uuid(my_uuid)?.ok_or_else(|| anyhow!("Failed to get account"))?;
    let moment_uuid = parse_uuid(Some(moment_uuid_str))?.ok_or_else(|| anyhow!("invalid uuid"))?;

    let select_sql = "SELECT m.uuid, m.author_uuid, m.content, m.visibility::int as visibility, \
        m.created_at, m.updated_at, bu.username, bu.icon, \
        (SELECT count(*) FROM biz_file_link bf WHERE bf.biz_id = m.uuid) as image_count, \
        (SELECT count(*) FROM moment_like ml WHERE ml.moment_uuid = m.uuid AND ml.is_del = false) as like_count, \
        (SELECT count(*) FROM moment_comment mc WHERE mc.moment_uuid = m.uuid AND mc.is_del = false) as comment_count, \
        (SELECT count(*) FROM moment_like ml2 WHERE ml2.moment_uuid = m.uuid AND ml2.user_uuid = ? AND ml2.is_del = false) as liked_by_me, \
        (SELECT count(*) FROM user_follow uf WHERE uf.follow_user_uuid = ? AND uf.target_user_uuid = m.author_uuid AND uf.is_del = false) as followed_by_me \
        FROM moment m JOIN basic_user bu ON m.author_uuid = bu.uuid \
        WHERE m.uuid = ? AND m.is_del = false AND (m.visibility = 0 OR m.author_uuid = ?)";
    let args = vec![
        value!(me.clone()),
        value!(me.clone()),
        value!(moment_uuid.clone()),
        value!(me.clone()),
    ];
    let row: Option<MomentRow> = rb.exec_decode(select_sql, args).await?;
    let Some(row) = row else {
        return Err(anyhow!("动态不存在"));
    };
    Ok(CommonResponseRef::<MomentVO>::success_json(&to_vo(row))?)
}

/// 点赞/取消点赞(切换)
pub async fn switch_like(
    rb: &RBatis,
    my_uuid: Option<String>,
    dto: LikeToggleDTO,
) -> Result<(), anyhow::Error> {
    let me = parse_uuid(my_uuid)?.ok_or_else(|| anyhow!("Failed to get account"))?;
    let moment_uuid = parse_uuid(Some(dto.moment_uuid))?.ok_or_else(|| anyhow!("invalid uuid"))?;
    ensure_moment_visible(rb, &moment_uuid, &me).await?;

    let now = get_now_time_stamp_as_secs()?;
    let existing = MomentLike::select_by_moment_and_user(rb, &moment_uuid, &me).await?;
    match existing {
        Some(mut like) => {
            like.is_del = Some(!like.is_del.unwrap_or(true));
            let id = like.id.clone().ok_or_else(|| anyhow!("like id missing"))?;
            MomentLike::update_by_map(rb, &like, value! {"id": id}).await?;
            Ok(())
        }
        None => {
            let id: Uuid = UuidNow::now_v7().to_string().parse()?;
            let like = MomentLike {
                id: Some(id),
                moment_uuid: Some(moment_uuid.clone()),
                user_uuid: Some(me.clone()),
                is_del: Some(false),
                created_at: Some(now),
            };
            MomentLike::insert(rb, &like).await?;
            Ok(())
        }
    }
}

/// 关注/取消关注(切换)
pub async fn switch_follow(
    rb: &RBatis,
    my_uuid: Option<String>,
    dto: FollowToggleDTO,
) -> Result<(), anyhow::Error> {
    let me = parse_uuid(my_uuid)?.ok_or_else(|| anyhow!("Failed to get account"))?;
    let target = parse_uuid(Some(dto.target_user_uuid))?.ok_or_else(|| anyhow!("invalid uuid"))?;
    if target == me {
        return Err(anyhow!("不能关注自己"));
    }

    let now = get_now_time_stamp_as_secs()?;
    let existing = UserFollow::select_by_follow_and_target(rb, &me, &target).await?;
    match existing {
        Some(mut follow) => {
            follow.is_del = Some(!follow.is_del.unwrap_or(true));
            let id = follow.id.clone().ok_or_else(|| anyhow!("follow id missing"))?;
            UserFollow::update_by_map(rb, &follow, value! {"id": id}).await?;
            Ok(())
        }
        None => {
            let id: Uuid = UuidNow::now_v7().to_string().parse()?;
            let follow = UserFollow {
                id: Some(id),
                follow_user_uuid: Some(me),
                target_user_uuid: Some(target),
                is_del: Some(false),
                created_at: Some(now),
            };
            UserFollow::insert(rb, &follow).await?;
            Ok(())
        }
    }
}

/// 删除动态(仅作者, 软删)
pub async fn delete_moment(
    rb: &RBatis,
    my_uuid: Option<String>,
    dto: DeleteMomentDTO,
) -> Result<(), anyhow::Error> {
    let me = parse_uuid(my_uuid)?.ok_or_else(|| anyhow!("Failed to get account"))?;
    let moment_uuid = parse_uuid(Some(dto.moment_uuid))?.ok_or_else(|| anyhow!("invalid uuid"))?;

    let moment = Moment::select_by_uuid(rb, &moment_uuid).await?;
    let Some(mut moment) = moment else {
        return Err(anyhow!("动态不存在"));
    };
    if moment.author_uuid.as_ref() != Some(&me) {
        return Err(anyhow!("无权操作"));
    }
    if moment.is_del.unwrap_or(false) {
        return Ok(());
    }
    moment.is_del = Some(true);
    moment.updated_at = Some(get_now_time_stamp_as_secs()?);
    let id = moment.uuid.clone().ok_or_else(|| anyhow!("moment uuid missing"))?;
    Moment::update_by_map(rb, &moment, value! {"uuid": id}).await?;
    Ok(())
}

/// 发表评论
pub async fn add_comment(
    rb: &RBatis,
    my_uuid: Option<String>,
    dto: AddCommentDTO,
) -> Result<String, anyhow::Error> {
    let me = parse_uuid(my_uuid)?.ok_or_else(|| anyhow!("Failed to get account"))?;
    let moment_uuid = parse_uuid(Some(dto.moment_uuid))?.ok_or_else(|| anyhow!("invalid uuid"))?;
    ensure_moment_visible(rb, &moment_uuid, &me).await?;

    let content = dto.content.trim().to_string();
    if content.is_empty() {
        return Err(anyhow!("评论内容不能为空"));
    }
    let content =
        if content.chars().count() > 1000 { content.chars().take(1000).collect() } else { content };

    let now = get_now_time_stamp_as_secs()?;
    let id: Uuid = UuidNow::now_v7().to_string().parse()?;
    let comment = MomentComment {
        id: Some(id.clone()),
        moment_uuid: Some(moment_uuid.clone()),
        author_uuid: Some(me.clone()),
        content: Some(content),
        is_del: Some(false),
        created_at: Some(now),
    };
    MomentComment::insert(rb, &comment).await?;

    get_comment_vo(rb, &id).await
}

async fn get_comment_vo(rb: &RBatis, id: &Uuid) -> Result<String, anyhow::Error> {
    let select_sql = "SELECT c.id, c.moment_uuid, c.author_uuid, c.content, c.created_at, \
        bu.username, bu.icon FROM moment_comment c JOIN basic_user bu ON c.author_uuid = bu.uuid \
        WHERE c.id = ?";
    let row: Option<MomentCommentRow> =
        rb.exec_decode(select_sql, vec![value!(id.clone())]).await?;
    let Some(row) = row else {
        return Err(anyhow!("评论不存在"));
    };
    Ok(CommonResponseRef::<MomentCommentVO>::success_json(&to_comment_vo(row))?)
}

/// 分页拉取评论
pub async fn get_comments(
    rb: &RBatis,
    my_uuid: Option<String>,
    moment_uuid_str: String,
    page_num: u32,
    page_size: u32,
) -> Result<String, anyhow::Error> {
    let me = parse_uuid(my_uuid)?.ok_or_else(|| anyhow!("Failed to get account"))?;
    let moment_uuid = parse_uuid(Some(moment_uuid_str))?.ok_or_else(|| anyhow!("invalid uuid"))?;
    ensure_moment_visible(rb, &moment_uuid, &me).await?;

    let page_num = page_num.max(1);
    let page_size = page_size.clamp(1, 50);
    let offset = (page_num as i64 - 1) * page_size as i64;

    let count_sql = "SELECT count(*) as count FROM moment_comment c WHERE c.moment_uuid = ? AND c.is_del = false";
    let count_row: Option<CountRow> =
        rb.exec_decode(count_sql, vec![value!(moment_uuid.clone())]).await?;
    let total = count_row.map(|r| r.count).unwrap_or(0) as u32;

    let select_sql = "SELECT c.id, c.moment_uuid, c.author_uuid, c.content, c.created_at, \
        bu.username, bu.icon FROM moment_comment c JOIN basic_user bu ON c.author_uuid = bu.uuid \
        WHERE c.moment_uuid = ? AND c.is_del = false \
        ORDER BY c.created_at DESC LIMIT ? OFFSET ?";
    let rows: Vec<MomentCommentRow> = rb
        .exec_decode(
            select_sql,
            vec![value!(moment_uuid.clone()), value!(page_size as i64), value!(offset)],
        )
        .await?;

    let list = rows.into_iter().map(to_comment_vo).collect();
    Ok(CommonResponseRef::<MomentCommentListVO>::success_json(&MomentCommentListVO {
        total,
        list,
    })?)
}

/// 分页拉取赞过的人
pub async fn get_like_list(
    rb: &RBatis,
    my_uuid: Option<String>,
    moment_uuid_str: String,
    page_num: u32,
    page_size: u32,
) -> Result<String, anyhow::Error> {
    let me = parse_uuid(my_uuid)?.ok_or_else(|| anyhow!("Failed to get account"))?;
    let moment_uuid = parse_uuid(Some(moment_uuid_str))?.ok_or_else(|| anyhow!("invalid uuid"))?;
    ensure_moment_visible(rb, &moment_uuid, &me).await?;

    let page_num = page_num.max(1);
    let page_size = page_size.clamp(1, 50);
    let offset = (page_num as i64 - 1) * page_size as i64;

    let count_sql = "SELECT count(*) as count FROM moment_like ml WHERE ml.moment_uuid = ? AND ml.is_del = false";
    let count_row: Option<CountRow> =
        rb.exec_decode(count_sql, vec![value!(moment_uuid.clone())]).await?;
    let total = count_row.map(|r| r.count).unwrap_or(0) as u32;

    let select_sql = "SELECT bu.uuid, bu.username, bu.icon, ml.created_at \
        FROM moment_like ml JOIN basic_user bu ON ml.user_uuid = bu.uuid \
        WHERE ml.moment_uuid = ? AND ml.is_del = false \
        ORDER BY ml.created_at DESC LIMIT ? OFFSET ?";
    let rows: Vec<MomentLikerRow> = rb
        .exec_decode(
            select_sql,
            vec![value!(moment_uuid.clone()), value!(page_size as i64), value!(offset)],
        )
        .await?;

    let list = rows.into_iter().map(to_liker_vo).collect();
    Ok(CommonResponseRef::<MomentLikerListVO>::success_json(&MomentLikerListVO { total, list })?)
}
