use anyhow::{anyhow, Result};
use rbatis::rbdc::Uuid;
use rbatis::RBatis;
use tracing::info;

use common::utils::time::get_now_time_stamp_as_millis;
use entity::models::group_entity::{
    group_info::GroupInfo,
    group_invitation::{GroupInvitation, INVITATION_ACCEPTED, INVITATION_DECLINED, INVITATION_PENDING},
    group_member::{GroupMember, ROLE_ADMIN, ROLE_MEMBER, ROLE_OWNER, STATUS_NORMAL},
    group_message_record::GroupMessageRecord,
};

use crate::http_service::group_service::group_dto::{
    create_group_dto::CreateGroupDTO,
    group_message_history_dto::GroupMessageHistoryDTO,
    invite_member_dto::{HandleInvitationDTO, InviteMemberDTO},
    set_role_dto::SetRoleDTO,
    update_group_dto::UpdateGroupDTO,
};
use crate::http_service::group_service::group_vo::{
    group_info_vo::{GroupInfoVO, GroupListItemVO},
    group_invitation_vo::GroupInvitationVO,
    group_member_vo::GroupMemberVO,
    group_message_vo::{GroupMessageVO, UnreadCountVO},
};
use crate::http_service::notify_service::service::system_notification::{
    send_group_invite_msg, send_group_invite_result_msg,
};

pub async fn create_group_service(
    rb: &RBatis,
    owner_uuid: &str,
    dto: CreateGroupDTO,
) -> Result<GroupInfoVO> {
    let now = get_now_time_stamp_as_millis()?;
    let group_uuid = Uuid::new();
    let owner = owner_uuid.parse::<Uuid>()?;

    let group_info = GroupInfo {
        id: None,
        group_uuid: Some(group_uuid.clone()),
        group_name: Some(dto.group_name),
        avatar: dto.avatar,
        owner_uuid: Some(owner.clone()),
        description: dto.description,
        max_members: dto.max_members.or(Some(500)),
        created_at: Some(now),
        updated_at: Some(now),
        status: Some(1),
    };

    GroupInfo::insert(rb, &group_info).await?;

    let group_member = GroupMember {
        id: None,
        group_uuid: Some(group_uuid.clone()),
        user_uuid: Some(owner.clone()),
        role: Some(ROLE_OWNER),
        nickname: None,
        join_time: Some(now),
        last_read_msg_id: Some(0),
        muted: Some(false),
        status: Some(STATUS_NORMAL),
    };

    GroupMember::insert(rb, &group_member).await?;

    info!("[群组] 创建成功 group_uuid={} owner={}", group_uuid, owner_uuid);

    Ok(GroupInfoVO {
        group_uuid: group_uuid.to_string(),
        group_name: group_info.group_name.unwrap(),
        avatar: group_info.avatar,
        owner_uuid: owner_uuid.to_string(),
        description: group_info.description,
        max_members: group_info.max_members.unwrap_or(500),
        member_count: 1,
        created_at: now,
        updated_at: now,
        status: 1,
    })
}

pub async fn get_group_info_service(rb: &RBatis, group_uuid: &str) -> Result<Option<GroupInfoVO>> {
    let uuid = group_uuid.parse::<Uuid>()?;
    let group = GroupInfo::select_by_group_uuid(rb, &uuid).await?;

    match group {
        Some(g) => {
            let member_count = count_group_members(rb, group_uuid).await?;
            Ok(Some(GroupInfoVO {
                group_uuid: g.group_uuid.map(|u: Uuid| u.to_string()).unwrap_or_default(),
                group_name: g.group_name.unwrap_or_default(),
                avatar: g.avatar,
                owner_uuid: g.owner_uuid.map(|u: Uuid| u.to_string()).unwrap_or_default(),
                description: g.description,
                max_members: g.max_members.unwrap_or(500),
                member_count,
                created_at: g.created_at.unwrap_or(0),
                updated_at: g.updated_at.unwrap_or(0),
                status: g.status.unwrap_or(1),
            }))
        }
        None => Ok(None),
    }
}

async fn count_group_members(rb: &RBatis, group_uuid: &str) -> Result<i64> {
    let uuid = group_uuid.parse::<Uuid>()?;
    let members: Vec<GroupMember> = GroupMember::select_members_by_group(rb, &uuid).await?;
    Ok(members.len() as i64)
}

pub async fn update_group_service(
    rb: &RBatis,
    user_uuid: &str,
    dto: UpdateGroupDTO,
) -> Result<bool> {
    let group_uuid = dto.group_uuid.parse::<Uuid>()?;
    let group = GroupInfo::select_by_group_uuid(rb, &group_uuid).await?;

    match group {
        Some(g) => {
            if g.owner_uuid.clone().map(|u: Uuid| u.to_string()) != Some(user_uuid.to_string()) {
                return Ok(false);
            }

            let now = get_now_time_stamp_as_millis()?;
            let mut update_group = g.clone();
            update_group.group_name = dto.group_name.or(update_group.group_name);
            update_group.avatar = dto.avatar.or(update_group.avatar);
            update_group.description = dto.description.or(update_group.description);
            update_group.updated_at = Some(now);

            GroupInfo::update_by_group_uuid(rb, &update_group, &group_uuid).await?;
            info!("[群组] 更新成功 group_uuid={}", group_uuid);
            Ok(true)
        }
        None => Ok(false),
    }
}

pub async fn dissolve_group_service(rb: &RBatis, user_uuid: &str, group_uuid: &str) -> Result<bool> {
    let uuid = group_uuid.parse::<Uuid>()?;
    let group = GroupInfo::select_by_group_uuid(rb, &uuid).await?;

    match group {
        Some(mut g) => {
            if g.owner_uuid.clone().map(|u: Uuid| u.to_string()) != Some(user_uuid.to_string()) {
                return Ok(false);
            }

            let now = get_now_time_stamp_as_millis()?;
            g.status = Some(2);
            g.updated_at = Some(now);
            GroupInfo::update_by_group_uuid(rb, &g, &uuid).await?;

            info!("[群组] 解散成功 group_uuid={}", group_uuid);
            Ok(true)
        }
        None => Ok(false),
    }
}

pub async fn get_my_groups_service(rb: &RBatis, user_uuid: &str) -> Result<Vec<GroupListItemVO>> {
    let uuid = user_uuid.parse::<Uuid>()?;
    let memberships: Vec<GroupMember> = GroupMember::select_groups_by_user(rb, &uuid).await?;

    let mut result = Vec::new();
    for membership in memberships {
        if let Some(g_uuid) = membership.group_uuid {
            if let Some(group) = GroupInfo::select_by_group_uuid(rb, &g_uuid).await? {
                let member_count = count_group_members(rb, &g_uuid.to_string()).await?;
                result.push(GroupListItemVO {
                    group_uuid: g_uuid.to_string(),
                    group_name: group.group_name.unwrap_or_default(),
                    avatar: group.avatar,
                    owner_uuid: group.owner_uuid.map(|u: Uuid| u.to_string()).unwrap_or_default(),
                    member_count,
                    last_msg_time: None,
                    unread_count: 0,
                });
            }
        }
    }

    Ok(result)
}

pub async fn get_group_members_service(rb: &RBatis, group_uuid: &str) -> Result<Vec<GroupMemberVO>> {
    let uuid = group_uuid.parse::<Uuid>()?;
    let members: Vec<GroupMember> = GroupMember::select_members_by_group(rb, &uuid).await?;

    Ok(members
        .into_iter()
        .map(|m| GroupMemberVO {
            user_uuid: m.user_uuid.map(|u: Uuid| u.to_string()).unwrap_or_default(),
            role: m.role.unwrap_or(0),
            nickname: m.nickname,
            join_time: m.join_time.unwrap_or(0),
            muted: m.muted.unwrap_or(false),
            status: m.status.unwrap_or(1),
        })
        .collect())
}

pub async fn invite_group_members_service(
    rb: &RBatis,
    operator_uuid: &str,
    dto: InviteMemberDTO,
) -> Result<Vec<String>> {
    let group_uuid = dto.group_uuid.parse::<Uuid>()?;
    let op_uuid = operator_uuid.parse::<Uuid>()?;

    let operator: Option<GroupMember> =
        GroupMember::select_by_group_and_user(rb, &group_uuid, &op_uuid).await?;

    match operator {
        Some(op) => {
            let role = op.role.unwrap_or(0);
            if role < ROLE_ADMIN {
                return Err(anyhow!("无权限邀请成员"));
            }

            let group = GroupInfo::select_by_group_uuid(rb, &group_uuid)
                .await?
                .ok_or(anyhow!("群组不存在"))?;
            let group_name = group.group_name.unwrap_or_default();

            let now = get_now_time_stamp_as_millis()?;
            let mut invited = Vec::new();

            for user_uuid_str in &dto.user_uuids {
                let user_uuid = user_uuid_str.parse::<Uuid>()?;

                // 检查是否已是成员
                let existing: Option<GroupMember> =
                    GroupMember::select_by_group_and_user(rb, &group_uuid, &user_uuid).await?;
                if existing.is_some() {
                    continue;
                }

                // 检查是否已有待处理的邀请
                let pending: Option<GroupInvitation> =
                    GroupInvitation::select_by_group_and_invitee(rb, &group_uuid, &user_uuid)
                        .await?;
                if let Some(ref p) = pending {
                    if p.status == Some(INVITATION_PENDING) {
                        continue;
                    }
                    // 更新旧邀请记录为待处理
                    let mut updated = p.clone();
                    updated.status = Some(INVITATION_PENDING);
                    updated.updated_at = Some(now);
                    GroupInvitation::update_by_id(rb, &updated, &p.id.unwrap()).await?;
                } else {
                    let invitation = GroupInvitation {
                        id: None,
                        group_uuid: Some(group_uuid.clone()),
                        inviter_uuid: Some(op_uuid.clone()),
                        invitee_uuid: Some(user_uuid.clone()),
                        status: Some(INVITATION_PENDING),
                        created_at: Some(now),
                        updated_at: Some(now),
                    };
                    GroupInvitation::insert(rb, &invitation).await?;
                }

                // 发送通知
                let notify_msg = format!("邀请你加入群聊「{}」", group_name);
                let _ = send_group_invite_msg(
                    rb,
                    user_uuid.clone(),
                    notify_msg,
                    Some(dto.group_uuid.clone()),
                )
                .await;

                invited.push(user_uuid_str.clone());
            }

            info!(
                "[群组] 邀请成员成功 group_uuid={} count={}",
                group_uuid,
                invited.len()
            );
            Ok(invited)
        }
        None => Err(anyhow!("操作者不是群成员")),
    }
}

pub async fn accept_group_invitation_service(
    rb: &RBatis,
    user_uuid: &str,
    dto: HandleInvitationDTO,
) -> Result<bool> {
    let group_uuid = dto.group_uuid.parse::<Uuid>()?;
    let u_uuid = user_uuid.parse::<Uuid>()?;

    let invitation: Option<GroupInvitation> =
        GroupInvitation::select_by_group_and_invitee(rb, &group_uuid, &u_uuid).await?;

    match invitation {
        Some(mut inv) if inv.status == Some(INVITATION_PENDING) => {
            let now = get_now_time_stamp_as_millis()?;

            // 更新邀请状态
            inv.status = Some(INVITATION_ACCEPTED);
            inv.updated_at = Some(now);
            GroupInvitation::update_by_id(rb, &inv, &inv.id.unwrap()).await?;

            // 添加为群成员
            let member = GroupMember {
                id: None,
                group_uuid: Some(group_uuid.clone()),
                user_uuid: Some(u_uuid.clone()),
                role: Some(ROLE_MEMBER),
                nickname: None,
                join_time: Some(now),
                last_read_msg_id: Some(0),
                muted: Some(false),
                status: Some(STATUS_NORMAL),
            };
            GroupMember::insert(rb, &member).await?;

            invalidate_group_member_cache(&dto.group_uuid).await?;

            // 通知邀请人
            let group = GroupInfo::select_by_group_uuid(rb, &group_uuid).await?;
            let group_name = group.and_then(|g| g.group_name).unwrap_or_default();
            let notify_msg = format!("用户已接受加入群聊「{}」的邀请", group_name);
            if let Some(inviter) = inv.inviter_uuid {
                let _ = send_group_invite_result_msg(
                    rb,
                    inviter,
                    notify_msg,
                    Some(dto.group_uuid.clone()),
                )
                .await;
            }

            info!(
                "[群组] 接受邀请 group_uuid={} user={}",
                dto.group_uuid, user_uuid
            );
            Ok(true)
        }
        _ => Ok(false),
    }
}

pub async fn decline_group_invitation_service(
    rb: &RBatis,
    user_uuid: &str,
    dto: HandleInvitationDTO,
) -> Result<bool> {
    let group_uuid = dto.group_uuid.parse::<Uuid>()?;
    let u_uuid = user_uuid.parse::<Uuid>()?;

    let invitation: Option<GroupInvitation> =
        GroupInvitation::select_by_group_and_invitee(rb, &group_uuid, &u_uuid).await?;

    match invitation {
        Some(mut inv) if inv.status == Some(INVITATION_PENDING) => {
            let now = get_now_time_stamp_as_millis()?;

            inv.status = Some(INVITATION_DECLINED);
            inv.updated_at = Some(now);
            GroupInvitation::update_by_id(rb, &inv, &inv.id.unwrap()).await?;

            info!(
                "[群组] 拒绝邀请 group_uuid={} user={}",
                dto.group_uuid, user_uuid
            );
            Ok(true)
        }
        _ => Ok(false),
    }
}

pub async fn get_pending_invitations_service(
    rb: &RBatis,
    user_uuid: &str,
) -> Result<Vec<GroupInvitationVO>> {
    let uuid = user_uuid.parse::<Uuid>()?;
    let invitations: Vec<GroupInvitation> =
        GroupInvitation::select_pending_by_invitee(rb, &uuid).await?;

    let mut result = Vec::new();
    for inv in invitations {
        if let (Some(g_uuid), Some(inviter_uuid)) = (inv.group_uuid, inv.inviter_uuid) {
            let group = GroupInfo::select_by_group_uuid(rb, &g_uuid).await?;
            let (group_name, group_avatar) = match group {
                Some(g) => (g.group_name.unwrap_or_default(), g.avatar),
                None => continue,
            };
            result.push(GroupInvitationVO {
                id: inv.id.unwrap_or(0),
                group_uuid: g_uuid.to_string(),
                group_name,
                group_avatar,
                inviter_uuid: inviter_uuid.to_string(),
                invitee_uuid: inv.invitee_uuid.map(|u| u.to_string()).unwrap_or_default(),
                status: inv.status.unwrap_or(INVITATION_PENDING),
                created_at: inv.created_at.unwrap_or(0),
            });
        }
    }

    Ok(result)
}

pub async fn get_sent_invitations_service(
    rb: &RBatis,
    user_uuid: &str,
) -> Result<Vec<GroupInvitationVO>> {
    let uuid = user_uuid.parse::<Uuid>()?;
    let invitations: Vec<GroupInvitation> =
        GroupInvitation::select_by_inviter(rb, &uuid).await?;

    let mut result = Vec::new();
    for inv in invitations {
        if let (Some(g_uuid), Some(inviter_uuid)) = (inv.group_uuid, inv.inviter_uuid) {
            let group = GroupInfo::select_by_group_uuid(rb, &g_uuid).await?;
            let (group_name, group_avatar) = match group {
                Some(g) => (g.group_name.unwrap_or_default(), g.avatar),
                None => continue,
            };
            result.push(GroupInvitationVO {
                id: inv.id.unwrap_or(0),
                group_uuid: g_uuid.to_string(),
                group_name,
                group_avatar,
                inviter_uuid: inviter_uuid.to_string(),
                invitee_uuid: inv.invitee_uuid.map(|u| u.to_string()).unwrap_or_default(),
                status: inv.status.unwrap_or(INVITATION_PENDING),
                created_at: inv.created_at.unwrap_or(0),
            });
        }
    }

    Ok(result)
}

pub async fn remove_group_member_service(
    rb: &RBatis,
    operator_uuid: &str,
    group_uuid: &str,
    target_uuid: &str,
) -> Result<bool> {
    let g_uuid = group_uuid.parse::<Uuid>()?;
    let op_uuid = operator_uuid.parse::<Uuid>()?;
    let operator: Option<GroupMember> = GroupMember::select_by_group_and_user(rb, &g_uuid, &op_uuid).await?;

    match operator {
        Some(op) => {
            let role = op.role.unwrap_or(0);
            if role < 1 {
                return Ok(false);
            }

            let t_uuid = target_uuid.parse::<Uuid>()?;
            let target: Option<GroupMember> = GroupMember::select_by_group_and_user(rb, &g_uuid, &t_uuid).await?;

            match target {
                Some(mut t) => {
                    if t.role.unwrap_or(0) >= role {
                        return Ok(false);
                    }
                    t.status = Some(3);
                    let user_uuid = t.user_uuid.clone();
                    GroupMember::update_by_group_and_user(rb, &t, &g_uuid, &user_uuid.unwrap()).await?;
                    invalidate_group_member_cache(group_uuid).await?;
                    info!("[群组] 移除成员成功 group_uuid={} target={}", group_uuid, target_uuid);
                    Ok(true)
                }
                None => Ok(false),
            }
        }
        None => Ok(false),
    }
}

pub async fn quit_group_service(rb: &RBatis, user_uuid: &str, group_uuid: &str) -> Result<bool> {
    let g_uuid = group_uuid.parse::<Uuid>()?;
    let u_uuid = user_uuid.parse::<Uuid>()?;
    let member: Option<GroupMember> = GroupMember::select_by_group_and_user(rb, &g_uuid, &u_uuid).await?;

    match member {
        Some(mut m) => {
            if m.role == Some(2) {
                return Ok(false);
            }
            m.status = Some(2);
            let user_uuid_val = m.user_uuid.clone();
            GroupMember::update_by_group_and_user(rb, &m, &g_uuid, &user_uuid_val.unwrap()).await?;
            invalidate_group_member_cache(group_uuid).await?;
            info!("[群组] 退出成功 group_uuid={} user={}", group_uuid, user_uuid);
            Ok(true)
        }
        None => Ok(false),
    }
}

pub async fn set_member_role_service(
    rb: &RBatis,
    operator_uuid: &str,
    dto: SetRoleDTO,
) -> Result<bool> {
    let group_uuid = dto.group_uuid.parse::<Uuid>()?;
    let op_uuid = operator_uuid.parse::<Uuid>()?;
    let operator: Option<GroupMember> = GroupMember::select_by_group_and_user(rb, &group_uuid, &op_uuid).await?;

    match operator {
        Some(op) => {
            if op.role != Some(2) {
                return Ok(false);
            }

            let t_uuid = dto.user_uuid.parse::<Uuid>()?;
            let target: Option<GroupMember> = GroupMember::select_by_group_and_user(rb, &group_uuid, &t_uuid).await?;

            match target {
                Some(mut t) => {
                    t.role = Some(dto.role);
                    let user_uuid = t.user_uuid.clone();
                    GroupMember::update_by_group_and_user(rb, &t, &group_uuid, &user_uuid.unwrap()).await?;
                    info!("[群组] 设置角色成功 group_uuid={} user={} role={}", dto.group_uuid, dto.user_uuid, dto.role);
                    Ok(true)
                }
                None => Ok(false),
            }
        }
        None => Ok(false),
    }
}

pub async fn get_group_message_history_service(
    rb: &RBatis,
    user_uuid: &str,
    dto: GroupMessageHistoryDTO,
) -> Result<Vec<GroupMessageVO>> {
    let group_uuid = dto.group_uuid.parse::<Uuid>()?;
    let u_uuid = user_uuid.parse::<Uuid>()?;
    
    let member: Option<GroupMember> = GroupMember::select_by_group_and_user(rb, &group_uuid, &u_uuid).await?;

    if member.is_none() {
        return Ok(Vec::new());
    }

    let start = dto.start.unwrap_or(0);
    let size = dto.size.unwrap_or(20);

    let messages: Vec<GroupMessageRecord> = GroupMessageRecord::select_by_group(rb, &group_uuid, start, size).await?;

    Ok(messages
        .into_iter()
        .map(|m| GroupMessageVO {
            nano_id: m.nano_id.unwrap_or_default(),
            group_uuid: m.group_uuid.map(|u: Uuid| u.to_string()).unwrap_or_default(),
            send_user: m.send_user.map(|u: Uuid| u.to_string()).unwrap_or_default(),
            timestamp: m.timestamp.unwrap_or(0),
            raw: m.raw.0.to_vec(),
            msg_type: m.msg_type.unwrap_or(1),
            recalled: m.recalled.unwrap_or(false),
        })
        .collect())
}

pub async fn get_unread_group_messages_service(
    rb: &RBatis,
    user_uuid: &str,
) -> Result<Vec<UnreadCountVO>> {
    let uuid = user_uuid.parse::<Uuid>()?;
    let memberships: Vec<GroupMember> = GroupMember::select_groups_by_user(rb, &uuid).await?;

    let mut result = Vec::new();
    for membership in memberships {
        if let (Some(g_uuid), Some(last_read_msg_id)) =
            (membership.group_uuid, membership.last_read_msg_id)
        {
            let unread: Vec<GroupMessageRecord> = GroupMessageRecord::select_unread(rb, &g_uuid, last_read_msg_id).await?;
            if !unread.is_empty() {
                result.push(UnreadCountVO {
                    group_uuid: g_uuid.to_string(),
                    unread_count: unread.len() as i64,
                });
            }
        }
    }

    Ok(result)
}

async fn invalidate_group_member_cache(group_uuid: &str) -> Result<()> {
    use deadpool_redis::redis::AsyncCommands;
    use common::REDIS_CLIENT;

    let cache_key = format!("group:members:{}", group_uuid);

    let redis = REDIS_CLIENT.read().await;
    if let Some(redis) = redis.as_ref() {
        if let Ok(mut conn) = redis.get().await {
            let _: Result<(), _> = conn.del(&cache_key).await;
        }
    }

    Ok(())
}
