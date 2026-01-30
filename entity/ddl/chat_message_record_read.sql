-- public.chat_message_record_read table definition

-- Drop table
-- DROP TABLE chat_message_record_read;

CREATE TABLE IF NOT EXISTS chat_message_record_read (
    id int4 DEFAULT nextval('chat_message_record_read_status_id_seq'::regclass) NOT NULL, -- 自增id
    send_user uuid NOT NULL, -- 发送人id
    recv_user uuid NOT NULL, -- 接收人id
    "timestamp" int8 NOT NULL, -- 创建时间
    nano_id varchar NOT NULL, -- 消息主键
    CONSTRAINT chat_message_record_read_status_pk PRIMARY KEY (id)
);

-- Table comment
COMMENT ON TABLE public.chat_message_record_read IS '聊天记录已读状态表';

-- Column comments
COMMENT ON COLUMN public.chat_message_record_read.id IS '自增id';
COMMENT ON COLUMN public.chat_message_record_read.send_user IS '发送人id';
COMMENT ON COLUMN public.chat_message_record_read.recv_user IS '接收人id';
COMMENT ON COLUMN public.chat_message_record_read."timestamp" IS '创建时间';
COMMENT ON COLUMN public.chat_message_record_read.nano_id IS '消息主键';
