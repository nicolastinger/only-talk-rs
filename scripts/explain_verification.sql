-- 任务09 §2: 分区行为 EXPLAIN 验证套件(开发者手动执行)
--
-- 用法:
--   psql "$DATABASE_URL" -f scripts/explain_verification.sql
--
-- 前置: 在开发库造足量测试数据(建议 10 万级单聊消息 / 百级会话),
--       可用 scripts/generate_test_accounts.sql 的账号扩展。
-- 说明: EXPLAIN 输出格式随 PG 版本变化, 不写自动断言 —— 结果记录进 PR。
--       V1/V2/V3/V6 期望单分区 + 索引扫描; V4/V5 是"已知代价实测"; V7 是遗留项决策输入。

\echo '=== V1: 单聊同步查询(任务05 核心路径) 期望 Index Only Scan idx_chat_msg_pull + 单分区 ==='
EXPLAIN (ANALYZE, BUFFERS)
SELECT * FROM chat_message_record
WHERE session_uuid = '00000000-0000-0000-0000-000000000001'
  AND id > 100 AND "timestamp" > 0
ORDER BY id ASC LIMIT 100;

\echo '=== V2: 窗口截断探测 期望同 V1(单分区 Index Only Scan) ==='
EXPLAIN (ANALYZE, BUFFERS)
SELECT 1 FROM chat_message_record
WHERE session_uuid = '00000000-0000-0000-0000-000000000001'
  AND id > 100 AND "timestamp" <= 0 LIMIT 1;

\echo '=== V3: 单聊翻历史(get_chat_record, 任务08) 期望 Index Scan + 单分区 ==='
EXPLAIN (ANALYZE, BUFFERS)
SELECT * FROM chat_message_record
WHERE session_uuid = '00000000-0000-0000-0000-000000000001'
ORDER BY id LIMIT 10 OFFSET 0;

\echo '=== V4: 聚合发现查询(任务03, 已知跨 16 分区 Append) 记录耗时; 秒级则走任务09 §2.4 LATERAL ==='
EXPLAIN (ANALYZE, BUFFERS)
SELECT DISTINCT ON (session_uuid) * FROM chat_message_record
WHERE recv_user = '00000000-0000-0000-0000-0000000000aa'
   OR send_user = '00000000-0000-0000-0000-0000000000aa'
ORDER BY session_uuid, id DESC;

\echo '=== V5: 清理删除(任务09 §3, 已知跨分区 Append) 记录单批耗时 ==='
EXPLAIN (ANALYZE, BUFFERS)
DELETE FROM chat_message_record
WHERE id IN (SELECT id FROM chat_message_record
             WHERE "timestamp" < 0 LIMIT 10000);

\echo '=== V6: 群消息同步(对照 V1) 期望 Index Only Scan idx_group_msg_pull + 单分区 ==='
EXPLAIN (ANALYZE, BUFFERS)
SELECT * FROM group_message_record
WHERE group_uuid = '00000000-0000-0000-0000-000000000001'
  AND id > 100 ORDER BY id ASC LIMIT 100;

\echo '=== V7: 会话列表(任务06, join + 内存排序) 目标 100 会话 < 50ms ==='
-- 完整 SQL 见 crates/entity/src/models/session_entity/session.rs 的 select_list_inner
EXPLAIN (ANALYZE, BUFFERS)
SELECT s.session_uuid, s.session_type, s.peer_uuid,
       c.last_message_id, c.last_message_at, c.last_preview, s.pinned, s.muted,
       (case s.session_type
          when 1 then (select count(*) from chat_message_record m
                       where m.session_uuid = s.session_uuid and m.id > s.last_read_id
                         and m.recv_user = '00000000-0000-0000-0000-0000000000aa')
          else (select count(*) from group_message_record m
                where m.group_uuid = s.session_uuid and m.id > s.last_read_id
                  and m.send_user <> '00000000-0000-0000-0000-0000000000aa')
        end) as unread
FROM user_session s
JOIN session c ON c.session_uuid = s.session_uuid
WHERE s.user_uuid = '00000000-0000-0000-0000-0000000000aa'
  AND (s.deleted_at is null or c.last_message_at > s.deleted_at)
ORDER BY s.pinned DESC, c.last_message_at DESC NULLS LAST, s.session_uuid DESC
LIMIT 50;
