-- public.chat_message_record_fail table definition

-- Drop table
-- DROP TABLE chat_message_record_fail;

CREATE TABLE IF NOT EXISTS chat_message_record_fail (
    id serial4 NOT NULL, -- 自增主键
    send_user uuid NOT NULL, -- 发送者id
    recv_user uuid NULL, -- 接收者id
    reason varchar NULL, -- 失败原因
    created_at varchar NULL, -- 创建时间
    nano_id varchar NULL, -- 消息主键
    CONSTRAINT chat_message_record_fail_pk PRIMARY KEY (id)
);

-- Table comment
COMMENT ON TABLE public.chat_message_record_fail IS '发送消息失败表';

-- Column comments
COMMENT ON COLUMN public.chat_message_record_fail.id IS '自增主键';
COMMENT ON COLUMN public.chat_message_record_fail.send_user IS '发送者id';
COMMENT ON COLUMN public.chat_message_record_fail.recv_user IS '接收者id';
COMMENT ON COLUMN public.chat_message_record_fail.reason IS '失败原因';
COMMENT ON COLUMN public.chat_message_record_fail.created_at IS '创建时间';
COMMENT ON COLUMN public.chat_message_record_fail.nano_id IS '消息主键';
