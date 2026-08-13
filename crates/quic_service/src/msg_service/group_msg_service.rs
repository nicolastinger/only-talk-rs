use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Result;
use common::config_str::{
    GROUP_MEMBERS_CACHE, MOBILE_PLATFORM, PC_PLATFORM, REDIS_INTERNAL_QUIC_SERVERS,
    REDIS_QUIC_SERVERS, REDIS_SPLIT,
};
use common::state::CoreState;
use common::utils::group_msg::{
    BroadcastType, GroupQuicMsg, InternalGroupBroadcast, InternalGroupBroadcastResponse,
};
use common::utils::internal_quic_client::make_internal_client_config;
use common::utils::text_msg::{HeadMsg, TextQuicMsg, X25, build_text_msg};
use common::utils::time::get_now_time_stamp_as_millis;
use dashmap::DashSet;
use deadpool_redis::redis::AsyncCommands;
use entity::models::group_entity::group_message_record::GroupMessageRecord;
use nanoid::nanoid;
use once_cell::sync::Lazy;
use quinn::Connection;
use rbatis::rbdc::{Bytes, Uuid};
use tracing::{debug, error, info, warn};

use crate::ConnectionsMap;
use crate::models::quic_connection::ConnectionType;

static DEDUP: Lazy<BroadcastDedup> = Lazy::new(BroadcastDedup::new);

type NodeAddressCache = Option<(Instant, Vec<(u32, std::net::SocketAddr)>)>;

/// 内部节点地址缓存(5 秒后过期,避免为每条群聊消息都扫描 Redis)
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
    // 转换为 TextQuicMsg 格式,以便客户端正确反序列化
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

pub async fn get_group_members_cached(core: &CoreState, group_uuid: &str) -> Result<Vec<String>> {
    let cache_key = format!("{}{}", GROUP_MEMBERS_CACHE, group_uuid).to_uppercase();

    let mut conn = core.redis.get().await?;
    let json: Option<String> = conn.get(&cache_key).await?;
    if let Some(json) = json
        && let Ok(members) = serde_json::from_str(&json)
    {
        return Ok(members);
    }

    let members = fetch_group_members_from_db(&core.db, group_uuid).await?;

    if let Ok(mut conn) = core.redis.get().await {
        let json = serde_json::to_string(&members)?;
        let _: Result<(), _> = conn.set(&cache_key, &json).await;
    }

    Ok(members)
}

async fn fetch_group_members_from_db(rb: &rbatis::RBatis, group_uuid: &str) -> Result<Vec<String>> {
    use entity::models::group_entity::group_member::GroupMember;

    let uuid = group_uuid.parse::<Uuid>()?;
    let members: Vec<GroupMember> = GroupMember::select_members_by_group(rb, &uuid).await?;

    Ok(members.into_iter().filter_map(|m| m.user_uuid.map(|u: Uuid| u.to_string())).collect())
}

pub async fn invalidate_group_member_cache(core: &CoreState, group_uuid: &str) -> Result<()> {
    let cache_key = format!("{}{}", GROUP_MEMBERS_CACHE, group_uuid).to_uppercase();

    let mut conn = core.redis.get().await?;
    let _: Result<(), _> = conn.del(&cache_key).await;

    Ok(())
}

pub async fn handle_group_msg_from_client(
    core: &CoreState,
    group_msg: GroupQuicMsg,
    server_index: u32,
    connections: &ConnectionsMap,
) -> Result<()> {
    let msg_bytes = serialize_group_msg(&group_msg)?;

    let all_members = get_group_members_cached(core, &group_msg.group_uuid).await?;
    debug!("[群聊] 成员缓存: {:?}", all_members);

    let sender_uuid: Uuid =
        group_msg.send_user.parse().map_err(|_| anyhow::anyhow!("Invalid sender UUID"))?;
    let sender_in_group =
        all_members.iter().any(|m| m.parse::<Uuid>().ok().as_ref() == Some(&sender_uuid));
    if !sender_in_group {
        return Err(anyhow::anyhow!(
            "Sender not in group members list sender={} group={} members={:?}",
            group_msg.send_user,
            group_msg.group_uuid,
            all_members
        ));
    }
    let group_msg_clone = group_msg.clone();
    let core_clone = core.clone();
    tokio::spawn(async move {
        if let Err(e) = save_group_message_to_db(&core_clone.db, &group_msg).await {
            error!("[群聊] 保存消息到数据库失败: {}", e);
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
            error!("[群聊] 本地广播处理失败: {}", e);
        }
    });

    let core_clone = core.clone();
    tokio::spawn(async move {
        match get_all_internal_node_addresses(&core_clone).await {
            Ok(nodes) => {
                for (node_index, addr) in &nodes {
                    if *node_index == server_index {
                        continue;
                    }
                    if let Err(e) = send_internal_group_broadcast(*addr, &broadcast).await {
                        error!("[群聊] 向节点 {} ({}) 发送广播失败: {}", node_index, addr, e);
                    }
                }
            }
            Err(e) => error!("[群聊] 获取节点地址失败: {}", e),
        }
    });

    Ok(())
}

async fn get_all_internal_node_addresses(
    core: &CoreState,
) -> Result<Vec<(u32, std::net::SocketAddr)>> {
    // 若缓存可用则直接返回
    {
        let cache_read = NODE_CACHE.lock().unwrap_or_else(|e| {
            error!("NODE_CACHE 锁中毒: {}", e);
            std::process::exit(1);
        });
        if let Some((ts, nodes)) = cache_read.as_ref()
            && ts.elapsed() < Duration::from_secs(5)
        {
            return Ok(nodes.clone());
        }
    }

    let mut conn = core.redis.get().await?;

    let pattern = format!("{}*", REDIS_INTERNAL_QUIC_SERVERS);
    let mut cursor: u64 = 0;
    let mut keys: Vec<String> = Vec::new();
    loop {
        let result: (u64, Vec<String>) = deadpool_redis::redis::cmd("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg(&pattern)
            .query_async(&mut conn)
            .await?;
        cursor = result.0;
        keys.extend(result.1);
        if cursor == 0 {
            break;
        }
    }

    let mut nodes = Vec::new();
    for key in keys {
        let addr_str: Option<String> = conn.get(&key).await?;
        if let Some(addr_str) = addr_str
            && let Ok(addr) = addr_str.parse::<std::net::SocketAddr>()
            && let Some(index_str) = key.strip_prefix(REDIS_INTERNAL_QUIC_SERVERS)
            && let Ok(index) = index_str.parse::<u32>()
        {
            nodes.push((index, addr));
        }
    }

    let mut cache_write = NODE_CACHE.lock().unwrap_or_else(|e| {
        error!("NODE_CACHE 写锁中毒: {}", e);
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
    let client_config = make_internal_client_config()?;
    let mut endpoint = quinn::Endpoint::client(std::net::SocketAddr::from(([0, 0, 0, 0], 0)))?;
    endpoint.set_default_client_config(client_config);

    info!("[群聊] 向 {} 发送广播,广播内容={:?}", addr, broadcast);
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
                        warn!("[群聊] 消息投递失败 member={} error={}", member, e);
                    } else if let Err(e) = send.finish().await {
                        warn!("[群聊] 结束发送失败 member={} error={}", member, e);
                    } else {
                        info!("[群聊] 消息投递成功 member={}", member);
                    }
                }
                Err(e) => {
                    warn!("[群聊] 打开 uni 流失败 member={} error={}", member, e);
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

pub fn find_online_connection(user_uuid: &str, connections: &ConnectionsMap) -> Option<Connection> {
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

async fn save_group_message_to_db(rb: &rbatis::RBatis, group_msg: &GroupQuicMsg) -> Result<()> {
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
    core: &CoreState,
    user_uuid: &str,
    connections: &ConnectionsMap,
) -> Result<()> {
    let rb = &core.db;
    use entity::models::group_entity::group_member::GroupMember;

    let uuid = user_uuid.parse::<Uuid>()?;
    let groups: Vec<GroupMember> = GroupMember::select_groups_by_user(rb, &uuid).await?;

    for group_member in groups {
        if let (Some(g_uuid), Some(last_read_msg_id)) =
            (group_member.group_uuid, group_member.last_read_msg_id)
        {
            let unread: Vec<GroupMessageRecord> =
                GroupMessageRecord::select_unread(rb, &g_uuid, last_read_msg_id).await?;

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

                    if let Ok(msg_bytes) = serialize_group_msg(&group_msg)
                        && let Some(conn) = find_online_connection(user_uuid, connections)
                        && let Ok(mut send) = conn.open_uni().await
                        && send.write_all(&msg_bytes).await.is_ok()
                    {
                        let _ = send.finish().await;
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
    let group_quic_msg =
        GroupQuicMsg { nano_id: nanoid!(), msg_type, group_uuid, send_user, raw, timestamp: now };
    serialize_group_msg(&group_quic_msg)
}

#[cfg(test)]
mod tests {
    use common::utils::group_msg::GroupQuicMsg;
    use common::utils::text_msg::{HeadMsg, TextQuicMsg, X25};

    use super::*;

    fn head_size() -> usize {
        let head = HeadMsg { version: 1, crc: 0, body_len: 0, message_type: 0 };
        bincode::serialize(&head).expect("序列化 head 失败").len()
    }

    fn make_group_msg() -> GroupQuicMsg {
        GroupQuicMsg {
            nano_id: "test-nano-1".to_string(),
            msg_type: 10, // GroupText
            group_uuid: "group-uuid-1".to_string(),
            send_user: "sender-uuid".to_string(),
            raw: b"hello group".to_vec(),
            timestamp: 1_700_000_000_000,
        }
    }

    #[test]
    fn test_serialize_group_msg_round_trip() {
        let group_msg = make_group_msg();
        let bytes = serialize_group_msg(&group_msg).expect("序列化群消息失败");

        // 解析出头部并校验 CRC(与 get_text_msg 相同的粘包协议)
        let head_len = head_size();
        let head: HeadMsg = bincode::deserialize(&bytes[..head_len]).expect("反序列化头部失败");
        let body: TextQuicMsg = bincode::deserialize(&bytes[head_len..]).expect("反序列化正文失败");

        assert_eq!(head.version, 1);
        assert_eq!(head.body_len as usize, bytes.len() - head_len);
        assert_eq!(head.message_type, group_msg.msg_type);
        assert_eq!(X25.checksum(&bytes[head_len..]), head.crc);

        // 业务字段完整保留
        assert_eq!(body.nano_id, group_msg.nano_id);
        assert_eq!(body.text_type, group_msg.msg_type);
        assert_eq!(body.raw, group_msg.raw);
        assert_eq!(body.recv_user, group_msg.group_uuid);
        assert_eq!(body.send_user, group_msg.send_user);
        assert_eq!(body.timestamp, group_msg.timestamp);
    }

    #[test]
    fn test_serialize_group_msg_crc_detects_corruption() {
        let group_msg = make_group_msg();
        let mut bytes = serialize_group_msg(&group_msg).expect("序列化群消息失败");
        let head_len = head_size();

        let original_crc = X25.checksum(&bytes[head_len..]);
        let head: HeadMsg = bincode::deserialize(&bytes[..head_len]).expect("反序列化头部失败");
        assert_eq!(head.crc, original_crc);

        // 篡改正文,CRC 应能检测到
        let last = bytes.len() - 1;
        bytes[last] ^= 0xFF;
        assert_ne!(X25.checksum(&bytes[head_len..]), original_crc);
    }

    #[test]
    fn test_generate_group_msg_produces_parseable_output() {
        let bytes = generate_group_msg(
            10,
            b"raw payload".to_vec(),
            "g-uuid".to_string(),
            "s-uuid".to_string(),
        )
        .expect("生成群消息失败");

        let head_len = head_size();
        let head: HeadMsg = bincode::deserialize(&bytes[..head_len]).expect("反序列化头部失败");
        let body: TextQuicMsg = bincode::deserialize(&bytes[head_len..]).expect("反序列化正文失败");

        assert_eq!(head.message_type, 10);
        assert_eq!(head.crc, X25.checksum(&bytes[head_len..]));
        assert!(!body.nano_id.is_empty());
        assert_eq!(body.raw, b"raw payload");
        assert_eq!(body.recv_user, "g-uuid");
        assert_eq!(body.send_user, "s-uuid");
    }

    #[tokio::test]
    async fn test_broadcast_dedup() {
        let dedup = BroadcastDedup::new();
        assert!(dedup.try_process("broadcast-1"));
        assert!(!dedup.try_process("broadcast-1"));
        assert!(dedup.try_process("broadcast-2"));
        assert!(!dedup.try_process("broadcast-2"));
    }

    #[test]
    fn test_find_online_connection_empty_map() {
        let connections = ConnectionsMap::default();
        assert!(find_online_connection("some-user", &connections).is_none());
    }
}
