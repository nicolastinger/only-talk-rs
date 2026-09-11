use std::str::FromStr;

use anyhow::anyhow;
use common::models::group_entity::group_info::GroupInfo;
use common::models::moment_entity::moment::Moment;
use common::models::moment_entity::moment_comment::MomentComment;
use common::models::plaza_entity::plaza_user_info::PlazaUserInfo;
use common::models::report_entity::report::{
    Report, STATUS_PENDING, TARGET_GROUP, TARGET_MOMENT, TARGET_MOMENT_COMMENT, TARGET_PLAZA_USER,
    TARGET_USER,
};
use common::models::user_entity::basic_user::BasicUser;
use common::utils::time::get_now_time_stamp_as_secs;
use rbatis::RBatis;
use rbatis::rbdc::Uuid;
use uuid::Uuid as UuidNow;

use crate::http_service::report_service::dto::report_dto::CreateReportDTO;
use crate::utils::http_response::CommonResponseRef;

/// 举报原因最大长度
const MAX_REASON_LEN: usize = 500;

fn parse_uuid(v: Option<String>) -> Result<Option<Uuid>, anyhow::Error> {
    match v {
        Some(s) => Ok(Some(Uuid::from_str(&s)?)),
        None => Ok(None),
    }
}

/// 校验举报目标存在且有效
async fn ensure_target_exists(
    rb: &RBatis,
    target_type: i16,
    target_uuid: &Uuid,
) -> Result<(), anyhow::Error> {
    match target_type {
        TARGET_USER => {
            if BasicUser::select_by_uuid(rb, target_uuid).await?.is_none() {
                return Err(anyhow!("举报目标不存在"));
            }
        }
        TARGET_GROUP => {
            if GroupInfo::select_by_group_uuid(rb, target_uuid).await?.is_none() {
                return Err(anyhow!("举报目标不存在"));
            }
        }
        TARGET_MOMENT => match Moment::select_by_uuid(rb, target_uuid).await? {
            Some(m) if !m.is_del.unwrap_or(false) => {}
            _ => return Err(anyhow!("举报目标不存在")),
        },
        TARGET_PLAZA_USER => {
            if PlazaUserInfo::select_by_uuid(rb, target_uuid).await?.is_none() {
                return Err(anyhow!("举报目标不存在"));
            }
        }
        TARGET_MOMENT_COMMENT => match MomentComment::select_by_id(rb, target_uuid).await? {
            Some(c) if !c.is_del.unwrap_or(false) => {}
            _ => return Err(anyhow!("举报目标不存在")),
        },
        _ => return Err(anyhow!("不支持的举报类型")),
    }
    Ok(())
}

/// 提交举报
pub async fn create_report(
    rb: &RBatis,
    my_uuid: Option<String>,
    dto: CreateReportDTO,
) -> Result<String, anyhow::Error> {
    let me = parse_uuid(my_uuid)?.ok_or_else(|| anyhow!("Failed to get account"))?;
    let target_uuid = parse_uuid(Some(dto.target_uuid))?.ok_or_else(|| anyhow!("invalid uuid"))?;

    let reason = dto.reason.trim().to_string();
    if reason.is_empty() {
        return Err(anyhow!("举报原因不能为空"));
    }
    let reason = if reason.chars().count() > MAX_REASON_LEN {
        reason.chars().take(MAX_REASON_LEN).collect()
    } else {
        reason
    };

    if (dto.target_type == TARGET_USER || dto.target_type == TARGET_PLAZA_USER) && target_uuid == me
    {
        return Err(anyhow!("不能举报自己"));
    }

    ensure_target_exists(rb, dto.target_type, &target_uuid).await?;

    // 同一举报人对同一目标的待处理举报去重
    if Report::select_pending_by_reporter_target(rb, &me, dto.target_type, &target_uuid)
        .await?
        .is_some()
    {
        return Err(anyhow!("你已举报过该内容，请等待处理"));
    }

    let now = get_now_time_stamp_as_secs()?;
    let uuid: Uuid = UuidNow::now_v7().to_string().parse()?;
    let report = Report {
        uuid: Some(uuid),
        reporter_uuid: Some(me),
        target_type: Some(dto.target_type),
        target_uuid: Some(target_uuid),
        reason: Some(reason),
        status: Some(STATUS_PENDING),
        is_del: Some(false),
        created_at: Some(now),
        updated_at: Some(now),
    };
    Report::insert(rb, &report).await?;

    Ok(CommonResponseRef::<bool>::success_json(&true)?)
}
