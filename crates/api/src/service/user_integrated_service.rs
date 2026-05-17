use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::net::SocketAddr;

use anyhow::anyhow;
use common::utils::message_types::NOTIFY_TYPE_MSG;
use common::utils::internal_quic_msg::{InternalQuicRequest, RequestSource};
use common::utils::server_count_sync::get_server_count;
use common::read_global_config;
use http_service::http_service::notify_service::service::system_notification::{
    send_process_friend_msg, send_request_friend_msg,
};
use http_service::http_service::user_service::dto::friend_request_info_dto::FriendRequestInfoDTO;
use http_service::http_service::user_service::service::friend_service::{
    add_friend, process_friend,
};
use http_service::utils::http_response::CommonResponseNoDataRef;
use rbatis::RBatis;
use common::utils::internal_quic_client::send_internal_quic_msg;

/// hash 取模计算首选节点序号
fn compute_preferred_index(uuid: &str) -> u32 {
    let sc = get_server_count();
    if sc <= 1 {
        return 0;
    }
    let mut hasher = DefaultHasher::new();
    uuid.hash(&mut hasher);
    (hasher.finish() as u32) % sc
}

pub async fn add_user_with_notify(
    rb: &RBatis,
    friend: FriendRequestInfoDTO,
) -> Result<String, anyhow::Error> {
    tracing::debug!("开始添加好友流程: request_user={:?}, accept_user={:?}", 
        friend.request_user, friend.accept_user);
    
    // 1 添加好友
    let friend_request = add_friend(rb, friend).await?;
    tracing::debug!("好友请求已创建: uuid={:?}, request_user={:?}, accept_user={:?}", 
        friend_request.uuid, friend_request.request_user, friend_request.accept_user);

    let target_uuid = friend_request.accept_user.ok_or(anyhow!("请选择一个用户"))?;
    let biz_id = friend_request.uuid.ok_or(anyhow!("添加好友失败，找不到请求id"))?.to_string();
    tracing::debug!("目标用户: target_uuid={}, biz_id={}", target_uuid, biz_id);

    // 2 发送系统通知 (落库)
    let quic_msg = send_request_friend_msg(
        rb,
        target_uuid,
        friend_request.request_message.clone().ok_or(anyhow!("请填写申请理由"))?,
        Some(biz_id.clone()),
    )
    .await?;
    tracing::debug!("系统通知已落库: quic_msg.user_id={:?}, request_message={:?}", 
        quic_msg.user_id, friend_request.request_message);
    
    let json_str: String = serde_json::to_string(&quic_msg)?;
    let target_id_str = quic_msg.user_id.ok_or(anyhow!("通知缺少目标用户ID"))?.to_string();
    tracing::debug!("通知JSON序列化完成: target_id={}, payload_length={}", target_id_str, json_str.len());

    // 3 通过内网QUIC服务转发通知
    let addr_str = read_global_config!("internal_quic_server", "address");
    let server_addr: SocketAddr = addr_str.parse()?;
    let preferred_index = compute_preferred_index(&target_id_str);
    tracing::debug!("QUIC服务配置: address={}, server_addr={}, preferred_index={}", 
        addr_str, server_addr, preferred_index);
    
    let request = InternalQuicRequest {
        msg_type: NOTIFY_TYPE_MSG,
        payload: json_str.clone(),
        target_user: target_id_str.clone(),
        preferred_index,
        platform: common::config_str::PC_PLATFORM.to_string(),
        source: RequestSource::HttpApi,
        ttl: 3,
    };
    tracing::debug!("准备发送QUIC消息: msg_type={}, target_user={}, platform={}, source={:?}, ttl={}", 
        request.msg_type, request.target_user, request.platform, request.source, request.ttl);
    
    send_internal_quic_msg(server_addr, request).await?;
    tracing::debug!("QUIC消息发送成功: target_user={}", target_id_str);

    Ok(CommonResponseNoDataRef::success_empty())
}

/// 处理好友申请
pub async fn process_friend_with_notify(
    rb: &RBatis,
    friend_request_info_dto: FriendRequestInfoDTO,
) -> Result<String, anyhow::Error> {
    // 1、处理好友申请
    let friend_request = process_friend(rb, friend_request_info_dto).await?;
    let target_uuid = friend_request.request_user.ok_or(anyhow!("请选择一个用户"))?;
    let biz_id = friend_request.uuid.ok_or(anyhow!("添加好友失败，找不到请求id"))?.to_string();
    // 2 发送系统通知 (落库)
    let accept_msg = friend_request
        .accept_message
        .filter(|m| !m.is_empty())
        .unwrap_or_else(|| "对方已处理您的好友申请".to_string());
    let quic_msg = send_process_friend_msg(
        rb,
        target_uuid,
        accept_msg,
        Some(biz_id),
    )
    .await?;
    let json_str: String = serde_json::to_string(&quic_msg)?;
    let target_id_str = quic_msg.user_id.ok_or(anyhow!("通知缺少目标用户ID"))?.to_string();

    // 3、通过内网QUIC服务转发通知
    let addr_str = read_global_config!("internal_quic_server", "address");
    let server_addr: SocketAddr = addr_str.parse()?;
    let preferred_index = compute_preferred_index(&target_id_str);
    let request = InternalQuicRequest {
        msg_type: NOTIFY_TYPE_MSG,
        payload: json_str,
        target_user: target_id_str,
        preferred_index,
        platform: common::config_str::PC_PLATFORM.to_string(),
        source: RequestSource::HttpApi,
        ttl: 3,
    };
    send_internal_quic_msg(server_addr, request).await?;

    Ok(CommonResponseNoDataRef::success_empty())
}

/// 获取分配给当前用户的外网 QUIC 节点地址（hash 取模）
pub async fn get_quic_server_for_user(uuid: &str) -> Result<String, anyhow::Error> {
    use deadpool_redis::redis::AsyncCommands;
    use common::config_str::REDIS_EXTERNAL_QUIC_SERVERS;
    use common::utils::server_count_sync::get_server_count;
    use http_service::utils::http_response::CommonResponseRef;
    use serde::Serialize;
    use tracing::info;

    #[derive(Serialize)]
    struct QuicServerInfo {
        index: u32,
        address: String,
    }

    let sc = get_server_count();
    let index = compute_preferred_index(uuid);
    info!("quic节点分配: server_count={} uuid={} index={}", sc, uuid, index);

    let redis = common::REDIS_CLIENT.read().await;
    let redis = redis.as_ref().ok_or(anyhow!("Redis 未初始化"))?;
    let mut conn = redis.get().await?;

    let key = format!("{}{}", REDIS_EXTERNAL_QUIC_SERVERS, index);
    let address: String = conn.get(&key).await?;

    let info = QuicServerInfo { index, address };
    CommonResponseRef::success_json(&info).map_err(|e| anyhow!("序列化失败: {}", e))
}
