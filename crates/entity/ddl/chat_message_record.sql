-- public.chat_message_record 表定义
-- 按会话哈希分区: 所有同步/未读查询都带 session_uuid, 单分区剪枝

-- 删除表
-- DROP TABLE chat_message_record;

CREATE TABLE IF NOT EXISTS chat_message_record (
    id           bigserial   NOT NULL,          -- 自增id(原 serial4, 消除21亿上限)
    session_uuid uuid        NOT NULL,          -- 会话标识: 单聊由用户对 v5 派生
    nano_id      varchar(64) NOT NULL,          -- 消息主键(会话内唯一)
    send_user    uuid        NOT NULL,          -- 发送人id
    recv_user    uuid        NOT NULL,          -- 接收用户id
    text_type    int4        NOT NULL DEFAULT 0, -- 消息类型，0-文本，1-图片
    "timestamp"  int8        NOT NULL,          -- 创建时间(毫秒)
    raw          bytea       NULL,              -- 二进制数据
    -- 分区表约束必须含分区键: nano_id 全局唯一性降级为会话内唯一,
    -- 跨会话由 nanoid!() 生成端保证 + 客户端去重兜底。
    -- 任务11: PK 以分区键打头, 一并承担同步/未读/最大id 三类热读;
    --         INCLUDE 使窗口过滤(timestamp)与未读计数(recv_user)免回表。
    CONSTRAINT chat_message_record_pk
        PRIMARY KEY (session_uuid, id) INCLUDE (recv_user, "timestamp"),
    CONSTRAINT chat_message_record_nano_uk UNIQUE (nano_id, session_uuid)
) PARTITION BY HASH (session_uuid);

-- 16 个哈希分区
DO $$
BEGIN
    FOR i IN 0..15 LOOP
        EXECUTE format(
            'CREATE TABLE IF NOT EXISTS chat_message_record_p%s
             PARTITION OF chat_message_record
             FOR VALUES WITH (MODULUS 16, REMAINDER %s)',
            lpad(i::text, 2, '0'), i);
    END LOOP;
END $$;

-- 任务11: 不再有任何独立索引
--   (原 idx_chat_msg_pull (session_uuid,id) INCLUDE(timestamp) → PK)
--   (原 idx_chat_msg_recv (recv_user,session_uuid,id)          → PK 的 INCLUDE 列)

COMMENT ON TABLE  public.chat_message_record               IS '单对单聊天记录(按会话哈希分区)';
COMMENT ON COLUMN public.chat_message_record.id           IS '自增id(bigserial)';
COMMENT ON COLUMN public.chat_message_record.session_uuid IS '会话标识, 单聊由用户对v5派生, 双向对称';
COMMENT ON COLUMN public.chat_message_record.nano_id      IS '消息主键(会话内唯一)';
COMMENT ON COLUMN public.chat_message_record.send_user    IS '发送人id';
COMMENT ON COLUMN public.chat_message_record.recv_user    IS '接收用户id';
COMMENT ON COLUMN public.chat_message_record.text_type    IS '消息类型，0-文本，1-图片';
COMMENT ON COLUMN public.chat_message_record."timestamp"  IS '创建时间(毫秒)';
COMMENT ON COLUMN public.chat_message_record.raw          IS '二进制数据';