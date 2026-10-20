//! session / user_session 数据访问层与聚合的集成测试。
//!
//! 使用独立测试库（默认 `only_talk_session_test`，可用 `TEST_DATABASE_NAME` 覆盖），
//! 与 `ddl_integration_test` 隔离，两个测试二进制可并行运行。
//! 测试启动时重建空库并应用全部 DDL，保证结果确定、可重复运行。
//!
//! 运行方式：
//!   cargo test -p entity --test session_dao_integration_test -- --ignored
//! 前提：本地 PostgreSQL 可用，且仓库根目录存在 `.env`。

use rbatis::RBatis;
use rbatis::rbdc::{Bytes, Uuid};
use rbs::value;

mod common;

use common::{
    DEFAULT_SESSION_TEST_DATABASE, admin_database_url, build_pool, init_tracing, recreate_database,
    test_database_name_or, test_database_url,
};
use entity::models::chat_entity::chat_message_record::ChatMessageRecord;
use entity::models::group_entity::group_info::GroupInfo;
use entity::models::group_entity::group_member::{GroupMember, ROLE_MEMBER, STATUS_NORMAL};
use entity::models::group_entity::group_message_record::{GroupMessageRecord, MSG_TYPE_TEXT};
use entity::models::session_entity::aggregate::aggregate_user_sessions;
use entity::models::session_entity::session::{SESSION_TYPE_GROUP, SESSION_TYPE_SINGLE, Session};
use entity::models::session_entity::user_session::UserSession;

/// 一次性建库 + 应用 DDL：由首个到达的用例完成，其余用例等待。
static DB_READY: tokio::sync::OnceCell<()> = tokio::sync::OnceCell::const_new();

/// 取得测试库连接池。
///
/// 连接池绑定创建它的 tokio 运行时：`#[tokio::test]` 每个用例各有独立运行时，
/// 因此不能跨用例共享同一个池（首个用例的运行时关闭后池即失效，报
/// "A Tokio 1.x context was found, but it is being shutdown"）。
/// 这里每个用例在自己的运行时内建池，并 `Box::leak` 成 `&'static` 以免改动用例签名。
async fn test_rb() -> &'static RBatis {
    DB_READY
        .get_or_init(|| async {
            init_tracing();
            let admin_url = admin_database_url().expect("读取测试库连接失败");
            let db_name = test_database_name_or(DEFAULT_SESSION_TEST_DATABASE);
            let admin = build_pool(&admin_url).await.expect("连接管理员库失败");
            recreate_database(&admin, &db_name).await.expect("重建测试库失败");
            let test_url = test_database_url(&admin_url, &db_name).expect("拼装测试库 URL 失败");
            let rb = build_pool(&test_url).await.expect("连接测试库失败");
            entity::ddl::apply_all_ddl(&rb).await.expect("应用 DDL 失败");
        })
        .await;

    let admin_url = admin_database_url().expect("读取测试库连接失败");
    let db_name = test_database_name_or(DEFAULT_SESSION_TEST_DATABASE);
    let test_url = test_database_url(&admin_url, &db_name).expect("拼装测试库 URL 失败");
    let rb = build_pool(&test_url).await.expect("连接测试库失败");
    Box::leak(Box::new(rb))
}

fn u(s: &str) -> Uuid {
    s.parse::<Uuid>().expect("解析 uuid 失败")
}

fn new_session(su: &Uuid, session_type: i16) -> Session {
    Session {
        session_uuid: su.clone(),
        session_type: Some(session_type),
        last_message_id: None,
        last_message_at: None,
        last_preview: None,
        created_at: None,
        updated_at: None,
    }
}

fn new_user_session(
    user: &Uuid,
    su: &Uuid,
    last_read: Option<i64>,
    synced: Option<i64>,
) -> UserSession {
    UserSession {
        id: None,
        user_uuid: user.clone(),
        session_uuid: su.clone(),
        session_type: Some(SESSION_TYPE_SINGLE),
        peer_uuid: None,
        last_read_id: last_read,
        synced_id: synced,
        pinned: None,
        muted: None,
        deleted_at: None,
        created_at: None,
        updated_at: None,
    }
}

async fn get_session(rb: &RBatis, su: &Uuid) -> Option<Session> {
    Session::select_by_map(rb, value! {"session_uuid": su})
        .await
        .expect("查询 session 失败")
        .into_iter()
        .next()
}

async fn get_user_session(rb: &RBatis, user: &Uuid, su: &Uuid) -> Option<UserSession> {
    UserSession::select_by_map(rb, value! {"user_uuid": user, "session_uuid": su})
        .await
        .expect("查询 user_session 失败")
        .into_iter()
        .next()
}

/// 执行单列标量查询（列别名固定为 `v`）
async fn scalar_i64(rb: &RBatis, sql: &str) -> i64 {
    let result: rbs::Value = rb.query(sql, vec![]).await.expect("标量查询失败");
    result
        .as_array()
        .and_then(|rows| rows.first())
        .and_then(|row| row.as_map())
        .map(|map| map.get(&rbs::Value::from("v")))
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
}

async fn insert_chat_msg(rb: &RBatis, su: &Uuid, from: &Uuid, to: &Uuid, ts: i64) {
    let msg = ChatMessageRecord {
        id: None,
        session_uuid: su.clone(),
        nano_id: Some(format!("nano-{}-{}", su, ts)),
        timestamp: Some(ts),
        raw: Bytes::from(vec![0x01, 0x02]),
        text_type: Some(1),
        send_user: from.clone(),
        recv_user: to.clone(),
    };
    ChatMessageRecord::insert(rb, &msg).await.expect("插入单聊消息失败");
}

async fn insert_group_info(rb: &RBatis, group_uuid: &Uuid, owner: &Uuid) {
    let info = GroupInfo {
        id: None,
        group_uuid: Some(group_uuid.clone()),
        group_name: Some("聚合测试群".to_string()),
        avatar: None,
        owner_uuid: Some(owner.clone()),
        description: None,
        max_members: Some(200),
        created_at: Some(0),
        updated_at: Some(0),
        status: Some(1),
    };
    GroupInfo::insert(rb, &info).await.expect("插入群信息失败");
}

async fn insert_group_member(rb: &RBatis, group_uuid: &Uuid, user: &Uuid) {
    let member = GroupMember {
        id: None,
        group_uuid: Some(group_uuid.clone()),
        user_uuid: Some(user.clone()),
        role: Some(ROLE_MEMBER),
        nickname: None,
        join_time: Some(0),
        muted: Some(false),
        status: Some(STATUS_NORMAL),
    };
    GroupMember::insert(rb, &member).await.expect("插入群成员失败");
}

async fn insert_group_msg(rb: &RBatis, group_uuid: &Uuid, from: &Uuid, ts: i64) {
    let msg = GroupMessageRecord {
        id: None,
        nano_id: Some(format!("gnano-{}-{}", group_uuid, ts)),
        group_uuid: Some(group_uuid.clone()),
        send_user: Some(from.clone()),
        timestamp: Some(ts),
        raw: Bytes::from(vec![0x03]),
        msg_type: Some(MSG_TYPE_TEXT),
        recalled: Some(false),
    };
    GroupMessageRecord::insert(rb, &msg).await.expect("插入群消息失败");
}

async fn max_chat_id(rb: &RBatis, su: &Uuid) -> i64 {
    scalar_i64(
        rb,
        &format!(
            "SELECT COALESCE(MAX(id), 0) AS v FROM chat_message_record WHERE session_uuid = '{}'",
            su
        ),
    )
    .await
}

async fn max_group_id(rb: &RBatis, group_uuid: &Uuid) -> i64 {
    scalar_i64(
        rb,
        &format!(
            "SELECT COALESCE(MAX(id), 0) AS v FROM group_message_record WHERE group_uuid = '{}'",
            group_uuid
        ),
    )
    .await
}

#[tokio::test]
#[ignore = "需要本地 PostgreSQL 与仓库根目录 .env"]
async fn session_upsert_idempotent() {
    let rb = test_rb().await;
    let su = u("00000000-0000-0000-0000-000000000a01");

    Session::upsert(rb, &new_session(&su, SESSION_TYPE_SINGLE)).await.expect("首次 upsert 失败");
    Session::update_last_message(rb, &su, 100, 1_000, Some("hi")).await.expect("更新失败");
    // 第二次 upsert 用不同 session_type, 但 DO NOTHING 不应覆盖已有字段
    Session::upsert(rb, &new_session(&su, SESSION_TYPE_GROUP)).await.expect("二次 upsert 失败");

    let rows = Session::select_by_map(rb, value! {"session_uuid": &su}).await.expect("查询失败");
    assert_eq!(rows.len(), 1, "upsert 幂等应只有 1 行");
    let s = rows.into_iter().next().expect("行存在");
    assert_eq!(s.session_type, Some(SESSION_TYPE_SINGLE), "已存在字段不应被覆盖");
    assert_eq!(s.last_message_id, Some(100));
}

#[tokio::test]
#[ignore = "需要本地 PostgreSQL 与仓库根目录 .env"]
async fn session_last_message_monotonic() {
    let rb = test_rb().await;
    let su = u("00000000-0000-0000-0000-000000000a02");

    Session::upsert(rb, &new_session(&su, SESSION_TYPE_SINGLE)).await.expect("upsert 失败");
    let advanced = Session::update_last_message(rb, &su, 100, 1_000, None).await.expect("更新失败");
    assert_eq!(advanced, 1, "更大的 id 应更新成功");
    let regressed = Session::update_last_message(rb, &su, 50, 500, None).await.expect("更新失败");
    assert_eq!(regressed, 0, "更小的 id 不应回退");

    let s = get_session(rb, &su).await.expect("session 应存在");
    assert_eq!(s.last_message_id, Some(100));
    assert_eq!(s.last_message_at, Some(1_000));
}

#[tokio::test]
#[ignore = "需要本地 PostgreSQL 与仓库根目录 .env"]
async fn aggregate_single_chat() {
    let rb = test_rb().await;
    let me = u("00000000-0000-0000-0000-000000000b01");
    let peer = u("00000000-0000-0000-0000-000000000b02");
    let su = u("00000000-0000-0000-0000-000000000b03");

    // 04b 语义: 好友通过即建双方 user_session 行 + session 行(聚合不再负责发现建行)
    Session::upsert(rb, &new_session(&su, SESSION_TYPE_SINGLE)).await.expect("session upsert 失败");
    let me_us = UserSession {
        id: None,
        user_uuid: me.clone(),
        session_uuid: su.clone(),
        session_type: Some(SESSION_TYPE_SINGLE),
        peer_uuid: Some(peer.clone()),
        last_read_id: None,
        synced_id: None,
        pinned: None,
        muted: None,
        deleted_at: None,
        created_at: None,
        updated_at: None,
    };
    UserSession::upsert(rb, &me_us).await.expect("me user_session upsert 失败");
    UserSession::upsert(
        rb,
        &UserSession { user_uuid: peer.clone(), peer_uuid: Some(me.clone()), ..me_us.clone() },
    )
    .await
    .expect("peer user_session upsert 失败");

    // 无消息时聚合跳过, 不建行/不清零
    let report_empty = aggregate_user_sessions(rb, &me).await.expect("空聚合失败");
    assert_eq!(report_empty.single_sessions, 0, "无消息会话应跳过");
    let s_empty = get_session(rb, &su).await.expect("session 行应存在");
    assert_eq!(s_empty.last_message_id, None, "无消息不应写入 last_message");

    insert_chat_msg(rb, &su, &peer, &me, 1_000).await;
    insert_chat_msg(rb, &su, &me, &peer, 2_000).await;
    let expected = max_chat_id(rb, &su).await;

    let report = aggregate_user_sessions(rb, &me).await.expect("聚合失败");
    assert_eq!(report.single_sessions, 1);

    let s = get_session(rb, &su).await.expect("session 应存在");
    assert_eq!(s.session_type, Some(SESSION_TYPE_SINGLE));
    assert_eq!(s.last_message_id, Some(expected), "应取该会话最新一条消息 id");

    // 聚合不重建 user_session 行, peer_uuid 保持 04b 建行时的值
    let us_me = get_user_session(rb, &me, &su).await.expect("me 的 user_session 应存在");
    assert_eq!(us_me.peer_uuid, Some(peer.clone()));

    // 幂等: 重复聚合结果一致
    let report2 = aggregate_user_sessions(rb, &me).await.expect("二次聚合失败");
    assert_eq!(report2.single_sessions, 1);

    let sessions =
        Session::select_by_map(rb, value! {"session_uuid": &su}).await.expect("查询失败");
    assert_eq!(sessions.len(), 1, "双方共享同一 session 行");
}

#[tokio::test]
#[ignore = "需要本地 PostgreSQL 与仓库根目录 .env"]
async fn aggregate_group() {
    let rb = test_rb().await;
    let me = u("00000000-0000-0000-0000-000000000c01");
    let group_uuid = u("00000000-0000-0000-0000-000000000c02");
    let sender = u("00000000-0000-0000-0000-000000000c03");

    insert_group_info(rb, &group_uuid, &sender).await;
    insert_group_member(rb, &group_uuid, &me).await;
    insert_group_msg(rb, &group_uuid, &sender, 1_000).await;
    insert_group_msg(rb, &group_uuid, &sender, 2_000).await;
    let expected = max_group_id(rb, &group_uuid).await;

    let report = aggregate_user_sessions(rb, &me).await.expect("聚合失败");
    assert_eq!(report.single_sessions, 0);
    assert_eq!(report.group_sessions, 1);

    let s = get_session(rb, &group_uuid).await.expect("群 session 应存在");
    assert_eq!(s.session_type, Some(SESSION_TYPE_GROUP));
    assert_eq!(s.last_message_id, Some(expected));

    let us = get_user_session(rb, &me, &group_uuid).await.expect("群 user_session 应存在");
    assert_eq!(us.session_type, Some(SESSION_TYPE_GROUP));
    assert_eq!(us.last_read_id, Some(expected), "入群初始化游标应到群最新消息");
    assert_eq!(us.synced_id, Some(expected));
}

#[tokio::test]
#[ignore = "需要本地 PostgreSQL 与仓库根目录 .env"]
async fn read_id_forward_and_clamped() {
    let rb = test_rb().await;
    let user = u("00000000-0000-0000-0000-000000000d01");
    let clamped = u("00000000-0000-0000-0000-000000000d02");
    let forward = u("00000000-0000-0000-0000-000000000d03");

    UserSession::upsert(rb, &new_user_session(&user, &clamped, None, Some(100)))
        .await
        .expect("upsert 失败");
    UserSession::upsert(rb, &new_user_session(&user, &forward, None, Some(100)))
        .await
        .expect("upsert 失败");

    // 钳制: 上报 150 超过 synced=100, 落在 100
    let affected =
        UserSession::update_last_read_id(rb, &user, &clamped, 150).await.expect("更新失败");
    assert_eq!(affected, 1);
    assert_eq!(
        get_user_session(rb, &user, &clamped).await.expect("行存在").last_read_id,
        Some(100),
        "应钳制到 synced_id"
    );

    // 不回退: 上报 80 小于当前 100
    let affected =
        UserSession::update_last_read_id(rb, &user, &clamped, 80).await.expect("更新失败");
    assert_eq!(affected, 0);
    assert_eq!(
        get_user_session(rb, &user, &clamped).await.expect("行存在").last_read_id,
        Some(100),
        "已读游标不应回退"
    );

    // 前进: 新行从 0 推进到 80 (仍在 synced 边界内)
    let affected =
        UserSession::update_last_read_id(rb, &user, &forward, 80).await.expect("更新失败");
    assert_eq!(affected, 1);
    assert_eq!(get_user_session(rb, &user, &forward).await.expect("行存在").last_read_id, Some(80));
}

#[tokio::test]
#[ignore = "需要本地 PostgreSQL 与仓库根目录 .env"]
async fn synced_id_forward_only() {
    let rb = test_rb().await;
    let user = u("00000000-0000-0000-0000-000000000e01");
    let su = u("00000000-0000-0000-0000-000000000e02");

    UserSession::upsert(rb, &new_user_session(&user, &su, None, None)).await.expect("upsert 失败");

    let advanced = UserSession::update_synced_id(rb, &user, &su, 100).await.expect("更新失败");
    assert_eq!(advanced, 1);
    assert_eq!(get_user_session(rb, &user, &su).await.expect("行存在").synced_id, Some(100));

    let regressed = UserSession::update_synced_id(rb, &user, &su, 50).await.expect("更新失败");
    assert_eq!(regressed, 0);
    assert_eq!(
        get_user_session(rb, &user, &su).await.expect("行存在").synced_id,
        Some(100),
        "同步游标不应回退"
    );
}

#[tokio::test]
#[ignore = "需要本地 PostgreSQL 与仓库根目录 .env"]
async fn soft_delete_advances_read() {
    let rb = test_rb().await;
    let user = u("00000000-0000-0000-0000-000000000f01");
    let su = u("00000000-0000-0000-0000-000000000f02");

    Session::upsert(rb, &new_session(&su, SESSION_TYPE_SINGLE)).await.expect("upsert 失败");
    Session::update_last_message(rb, &su, 200, 2_000, None).await.expect("更新失败");
    UserSession::upsert(rb, &new_user_session(&user, &su, None, None)).await.expect("upsert 失败");

    let affected = UserSession::soft_delete(rb, &user, &su).await.expect("软删失败");
    assert_eq!(affected, 1);

    let us = get_user_session(rb, &user, &su).await.expect("行存在");
    assert!(us.deleted_at.is_some(), "deleted_at 应被置位");
    assert_eq!(us.last_read_id, Some(200), "软删应把已读游标推到底");
}

#[tokio::test]
#[ignore = "需要本地 PostgreSQL 与仓库根目录 .env"]
async fn group_join_initializes_cursor() {
    let rb = test_rb().await;
    let user = u("00000000-0000-0000-0000-000000000901");
    let group_uuid = u("00000000-0000-0000-0000-000000000902");
    let sender = u("00000000-0000-0000-0000-000000000903");

    insert_group_info(rb, &group_uuid, &sender).await;
    insert_group_msg(rb, &group_uuid, &sender, 1_000).await;
    let expected = max_group_id(rb, &group_uuid).await;

    UserSession::init_for_group_join(rb, &user, &group_uuid).await.expect("初始化失败");
    let us = get_user_session(rb, &user, &group_uuid).await.expect("行存在");
    assert_eq!(us.last_read_id, Some(expected));
    assert_eq!(us.synced_id, Some(expected));

    // 再次调用应幂等: 值不变且只有 1 行
    UserSession::init_for_group_join(rb, &user, &group_uuid).await.expect("重复初始化失败");
    let us2 = get_user_session(rb, &user, &group_uuid).await.expect("行存在");
    assert_eq!(us2.last_read_id, Some(expected));
    assert_eq!(us2.synced_id, Some(expected));

    let count = scalar_i64(
        rb,
        &format!(
            "SELECT count(*) AS v FROM user_session WHERE user_uuid = '{}' AND session_uuid = '{}'",
            user, group_uuid
        ),
    )
    .await;
    assert_eq!(count, 1, "幂等初始化应只有 1 行");
}
