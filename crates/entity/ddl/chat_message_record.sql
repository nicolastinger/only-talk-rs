-- public.chat_message_record table definition

-- Drop table
-- DROP TABLE chat_message_record;

CREATE TABLE IF NOT EXISTS chat_message_record (
    send_user uuid NOT NULL, -- 发送人id
    recv_user uuid NOT NULL, -- 接收用户id
    text_type int4 DEFAULT 0 NOT NULL, -- 消息类型，0-文本，1-图片
    "timestamp" int8 NOT NULL, -- 创建时间
    id serial4 NOT NULL, -- 自增id
    nano_id varchar NOT NULL, -- 消息主键
    raw bytea NULL, -- 二进制数据
    CONSTRAINT chat_message_record_unique UNIQUE (id)
);

-- Indexes
CREATE INDEX chat_message_record_create_at_idx ON public.chat_message_record USING btree ("timestamp");
CREATE INDEX chat_message_record_send_user_idx ON public.chat_message_record USING btree (send_user, recv_user, "timestamp");

-- Table comment
COMMENT ON TABLE public.chat_message_record IS '单对单聊天列表';

-- Column comments
COMMENT ON COLUMN public.chat_message_record.send_user IS '发送人id';
COMMENT ON COLUMN public.chat_message_record.recv_user IS '接收用户id';
COMMENT ON COLUMN public.chat_message_record.text_type IS '消息类型，0-文本，1-图片';
COMMENT ON COLUMN public.chat_message_record."timestamp" IS '创建时间';
COMMENT ON COLUMN public.chat_message_record.id IS '自增id';
COMMENT ON COLUMN public.chat_message_record.nano_id IS '消息主键';
COMMENT ON COLUMN public.chat_message_record.raw IS '二进制数据';
