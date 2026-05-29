use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use anyhow::Result;
use dashmap::DashSet;
use deadpool_redis::redis::AsyncCommands;
use nanoid::nanoid;
use once_cell::sync::Lazy;
use quinn::Connection;
use rbatis::rbdc::{Bytes, Uuid};
use tracing::{debug, error, info, warn};

use common::config_str::{MOBILE_PLATFORM, PC_PLATFORM, REDIS_INTERNAL_QUIC_SERVERS, REDIS_QUIC_SERVERS, REDIS_SPLIT};
use common::utils::group_msg::{
    BroadcastType, GroupQuicMsg, InternalGroupBroadcast, InternalGroupBroadcastResponse,
};
use common::utils::time::get_now_time_stamp_as_millis;
use common::REDIS_CLIENT;
use entity::models::group_entity::group_message_record::GroupMessageRecord;

use crate::models::quic_connection::ConnectionType;
use crate::models::text_msg::{HeadMsg, TextQuicMsg};
use crate::msg_service::text_msg_service::build_text_msg;
use crate::{ConnectionsMap, X25};

static DEDUP: Lazy<BroadcastDedup> = Lazy::new(BroadcastDedup::new);

type NodeAddressCache = Option<(Instant, Vec<(u32, std::net::SocketAddr)>)>;

/// 内网节点地址缓存（5s 过期，避免每条群消息都扫 Redis）
static NODE_CACHE: Lazy<Mutex<NodeAddressCache>> = Lazy::new(|| Mutex::new(None));

pub struct BroadcastDedup {
    set: Arc<DashSet<String>>,
}

impl BroadcastDedup {
    pub fn new() -> Self {
        let set = Arc::new(DashSet::new());

        let cloned = set.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(60)).await;
                cloned.clear();
            }
        });

        Self { set }
    }

    pub fn try_process(&self, broadcast_id: &str) -> bool {
        self.set.insert(broadcast_id.to_string())
    }
}

impl Default for BroadcastDedup {
    fn default() -> Self {
        Self::new()
    }
}

pub fn serialize_group_msg(group_msg: &GroupQuicMsg) -> Result<Vec<u8>> {
    // 转换为 TextQuicMsg 格式，客户端才能正确反序列化
    let text_msg = TextQuicMsg {
        nano_id: group_msg.nano_id.clone(),
        text_type: group_msg.msg_type,
        raw: group_msg.raw.clone(),
        recv_user: group_msg.group_uuid.clone(),
        send_user: group_msg.send_user.clone(),
        timestamp: group_msg.timestamp,
    };
    let meta_data = bincode::serialize(&text_msg)?;
    let crc = X25.checksum(&meta_data);
    let head_msg = HeadMsg {
        version: 1,
        crc,
        body_len: meta_data.len() as u32,
        message_type: group_msg.msg_type,
    };

    build_text_msg(&head_msg, &text_msg)
}

pub async fn get_group_members_cached(group_uuid: &str) -> Result<Vec<String>> {
    let cache_key = format!("group:members:{}", group_uuid);

    let redis = REDIS_CLIENT.read().await;
    if let Some(redis) = redis.as_ref() {
        let mut conn = redis.get().await?;
        let json: Option<String> = conn.get(&cache_key).await?;
        if let Some(json) = json {
            if let Ok(members) = serde_json::from_str(&json) {
                return Ok(members);
            }
        }
    }

    let members = fetch_group_members_from_db(group_uuid).await?;

    if let Some(redis) = REDIS_CLIENT.read().await.as_ref() {
        if let Ok(mut conn) = redis.get().await {
            let json = serde_json::to_string(&members)?;
            let _: Result<(), _> = conn.set(&cache_key, &json).await;
        }
    }

    Ok(members)
}

async fn fetch_group_members_from_db(group_uuid: &str) -> Result<Vec<String>> {
    use common::RBATIS_DATABASE;
    use entity::models::group_entity::group_member::GroupMember;

    let rb = RBATIS_DATABASE.read().await;
    let rb = rb
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("数据库连接失败"))?;

    let uuid = group_uuid.parse::<Uuid>()?;
    let members: Vec<GroupMember> = GroupMember::select_members_by_group(rb, &uuid).await?;

    Ok(members
        .into_iter()
        .filter_map(|m| m.user_uuid.map(|u: Uuid| u.to_string()))
        .collect())
}

pub async fn invalidate_group_member_cache(group_uuid: &str) -> Result<()> {
    let cache_key = format!("group:members:{}", group_uuid);

    let redis = REDIS_CLIENT.read().await;
    if let Some(redis) = redis.as_ref() {
        let mut conn = redis.get().await?;
        let _: Result<(), _> = conn.del(&cache_key).await;
    }

    Ok(())
}

pub async fn handle_group_msg_from_client(
    group_msg: GroupQuicMsg,
    server_index: u32,
    connections: &ConnectionsMap,
) -> Result<()> {
    let msg_bytes = serialize_group_msg(&group_msg)?;

    let all_members = get_group_members_cached(&group_msg.group_uuid).await?;
    debug!("group members cache: {:?}", all_members);

    let sender_uuid: Uuid = group_msg
        .send_user
        .parse()
        .map_err(|_| anyhow::anyhow!("无效的发送者UUID"))?;
    let sender_in_group = all_members
        .iter()
        .any(|m| m.parse::<Uuid>().ok().as_ref() == Some(&sender_uuid));
    if !sender_in_group {
        return Err(anyhow::anyhow!(
            "发送者不在群成员列表中 sender={} group={} members={:?}",
            group_msg.send_user,
            group_msg.group_uuid,
            all_members
        ));
    }
    let group_msg_clone = group_msg.clone();
    tokio::spawn(async move {
        if let Err(e) = save_group_message_to_db(&group_msg).await {
            error!("保存群消息到数据库失败: {}", e);
        }
    });

    let broadcast = InternalGroupBroadcast {
        broadcast_type: BroadcastType::from_msg_type(group_msg_clone.msg_type),
        group_uuid: group_msg_clone.group_uuid,
        msg_bytes,
        sender: group_msg_clone.send_user,
        all_members,
        source_node: server_index,
        timestamp: group_msg_clone.timestamp,
        broadcast_id: group_msg_clone.nano_id,
    };
    
    let broadcast_clone = broadcast.clone();
    let connections_clone = connections.clone();
    
    tokio::spawn(async move {
        if let Err(e) = process_group_broadcast_local(&broadcast_clone, &connections_clone).await {
            error!("处理群消息失败: {}", e);
        }
    });


    tokio::spawn(async move {
        match get_all_internal_node_addresses().await {
            Ok(nodes) => {
                for (node_index, addr) in &nodes {
                    if *node_index == server_index {
                        continue;
                    }
                    let _ = send_internal_group_broadcast(*addr, &broadcast).await;
                }
            }
            Err(e) => error!("获取节点地址失败: {}", e),
        }
    });
    
    Ok(())
}

async fn get_all_internal_node_addresses() -> Result<Vec<(u32, std::net::SocketAddr)>> {
    // 命中缓存直接返回
    let cache_read = NODE_CACHE.lock().unwrap_or_else(|e| {
        error!("NODE_CACHE 锁中毒: {}", e);
        std::process::exit(1);
    });
    if let Some((ts, nodes)) = cache_read.as_ref() {
        if ts.elapsed() < Duration::from_secs(5) {
            return Ok(nodes.clone());
        }
    }
    drop(cache_read);

    let redis = REDIS_CLIENT.read().await;
    let redis = redis
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Redis 未初始化"))?;
    let mut conn = redis.get().await?;

    let pattern = format!("{}*", REDIS_INTERNAL_QUIC_SERVERS);
    let keys: Vec<String> = conn.keys(&pattern).await?;

    let mut nodes = Vec::new();
    for key in keys {
        let addr_str: Option<String> = conn.get(&key).await?;
        if let Some(addr_str) = addr_str {
            if let Ok(addr) = addr_str.parse::<std::net::SocketAddr>() {
                if let Some(index_str) = key.strip_prefix(REDIS_INTERNAL_QUIC_SERVERS) {
                    if let Ok(index) = index_str.parse::<u32>() {
                        nodes.push((index, addr));
                    }
                }
            }
        }
    }

    let mut cache_write = NODE_CACHE.lock().unwrap_or_else(|e| {
        error!("NODE_CACHE 写入锁中毒: {}", e);
        std::process::exit(1);
    });
    *cache_write = Some((Instant::now(), nodes.clone()));
    drop(cache_write);
    Ok(nodes)
}

async fn send_internal_group_broadcast(
    addr: std::net::SocketAddr,
    broadcast: &InternalGroupBroadcast,
) -> Result<InternalGroupBroadcastResponse> {
    let endpoint = quinn::Endpoint::client(std::net::SocketAddr::from(([0, 0, 0, 0], 0)))?;

    let conn = endpoint.connect(addr, "localhost")?.await?;

    let (mut send, mut recv) = conn.open_bi().await?;

    let body = bincode::serialize(broadcast)?;
    send.write_all(&body).await?;
    send.finish().await?;

    let mut buf = vec![0u8; 1024];
    let len = recv.read(&mut buf).await?.unwrap_or(0);
    let resp: InternalGroupBroadcastResponse = bincode::deserialize(&buf[..len])?;

    Ok(resp)
}

pub async fn process_group_broadcast_local(
    broadcast: &InternalGroupBroadcast,
    connections: &ConnectionsMap,
) -> Result<()> {
    if !DEDUP.try_process(&broadcast.broadcast_id) {
        return Ok(());
    }

    for member in &broadcast.all_members {
        if *member == broadcast.sender {
            continue;
        }

        if let Some(conn) = find_online_connection(member, connections) {
            match conn.open_uni().await {
                Ok(mut send) => {
                    if let Err(e) = send.write_all(&broadcast.msg_bytes).await {
                        warn!("[群聊广播] 投递失败 member={} error={}", member, e);
                    } else if let Err(e) = send.finish().await {
                        warn!("[群聊广播] finish失败 member={} error={}", member, e);
                    } else {
                        info!("[群聊广播] 投递成功 member={}", member);
                    }
                }
                Err(e) => {
                    warn!("[群聊广播] 打开uni流失败 member={} error={}", member, e);
                }
            }
        }
    }

    Ok(())
}

pub async fn process_group_broadcast(
    broadcast: &InternalGroupBroadcast,
    connections: &ConnectionsMap,
) -> Result<()> {
    if !DEDUP.try_process(&broadcast.broadcast_id) {
        return Ok(());
    }

    for member in &broadcast.all_members {
        if *member == broadcast.sender {
            continue;
        }

        if let Some(conn) = find_online_connection(member, connections) {
            let mut send = conn.open_uni().await?;
            send.write_all(&broadcast.msg_bytes).await?;
            send.finish().await?;
        }
    }

    Ok(())
}

pub fn find_online_connection(
    user_uuid: &str,
    connections: &ConnectionsMap,
) -> Option<Connection> {
    for platform in [PC_PLATFORM, MOBILE_PLATFORM] {
        let key = format!(
            "{}:{}{}{}{}",
            platform,
            REDIS_QUIC_SERVERS,
            user_uuid,
            REDIS_SPLIT,
            ConnectionType::Text,
        )
        .to_uppercase();

        if let Some(entry) = connections.get(&key) {
            return Some(entry.conn.clone());
        }
    }
    None
}

async fn save_group_message_to_db(group_msg: &GroupQuicMsg) -> Result<()> {
    use common::RBATIS_DATABASE;

    let rb = RBATIS_DATABASE.read().await;
    let rb = rb
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("数据库连接失败"))?;

    let record = GroupMessageRecord {
        id: None,
        nano_id: Some(group_msg.nano_id.clone()),
        group_uuid: Some(group_msg.group_uuid.parse::<Uuid>()?),
        send_user: Some(group_msg.send_user.parse::<Uuid>()?),
        timestamp: Some(group_msg.timestamp),
        raw: Bytes::from(group_msg.raw.clone()),
        msg_type: Some(group_msg.msg_type as i16),
        recalled: Some(false),
    };

    GroupMessageRecord::insert(rb, &record).await?;
    info!("[群聊] 消息已持久化 nano_id={}", group_msg.nano_id);

    Ok(())
}

pub async fn sync_offline_group_messages(
    user_uuid: &str,
    connections: &ConnectionsMap,
) -> Result<()> {
    use common::RBATIS_DATABASE;
    use entity::models::group_entity::group_member::GroupMember;

    let rb = RBATIS_DATABASE.read().await;
    let rb = rb
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("数据库连接失败"))?;

    let uuid = user_uuid.parse::<Uuid>()?;
    let groups: Vec<GroupMember> = GroupMember::select_groups_by_user(rb, &uuid).await?;

    for group_member in groups {
        if let (Some(g_uuid), Some(last_read_msg_id)) =
            (group_member.group_uuid, group_member.last_read_msg_id)
        {
            let unread: Vec<GroupMessageRecord> = GroupMessageRecord::select_unread(rb, &g_uuid, last_read_msg_id).await?;

            for msg in unread {
                if let (Some(nano_id), Some(send_user), Some(timestamp), Some(msg_type)) =
                    (msg.nano_id, msg.send_user, msg.timestamp, msg.msg_type)
                {
                    let group_msg = GroupQuicMsg {
                        nano_id,
                        msg_type: msg_type as u16,
                        group_uuid: g_uuid.to_string(),
                        send_user: send_user.to_string(),
                        raw: msg.raw.0.to_vec(),
                        timestamp,
                    };

                    if let Ok(msg_bytes) = serialize_group_msg(&group_msg) {
                        if let Some(conn) = find_online_connection(user_uuid, connections) {
                            if let Ok(mut send) = conn.open_uni().await {
                                if send.write_all(&msg_bytes).await.is_ok() {
                                    let _ = send.finish().await;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

pub fn generate_group_msg(
    msg_type: u16,
    raw: Vec<u8>,
    group_uuid: String,
    send_user: String,
) -> Result<Vec<u8>> {
    let now = get_now_time_stamp_as_millis()?;
    let group_quic_msg = GroupQuicMsg {
        nano_id: nanoid!(),
        msg_type,
        group_uuid,
        send_user,
        raw,
        timestamp: now,
    };
    serialize_group_msg(&group_quic_msg)
}
