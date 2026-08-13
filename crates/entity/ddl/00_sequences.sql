-- Sequences for v1.0.0
-- 仅保留建表时通过 nextval(...) 显式引用的序列；
-- 使用 serial4/bigserial 的表会由 PostgreSQL 自动创建同名序列，无需在此定义。

-- chat_message_record_read.id 使用 nextval('chat_message_record_read_status_id_seq'::regclass)
CREATE SEQUENCE IF NOT EXISTS chat_message_record_read_status_id_seq
    INCREMENT BY 1
    MINVALUE 1
    MAXVALUE 2147483647
    START 1
    CACHE 1
    NO CYCLE;

-- group_message_record_read.id 使用 nextval('group_message_record_read_id_seq'::regclass)
CREATE SEQUENCE IF NOT EXISTS group_message_record_read_id_seq
    INCREMENT BY 1
    MINVALUE 1
    MAXVALUE 2147483647
    START 1
    CACHE 1
    NO CYCLE;
