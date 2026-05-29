-- public.group_message_record_read table definition

-- Drop table
-- DROP TABLE group_message_record_read;

CREATE TABLE IF NOT EXISTS group_message_record_read (
    id int4 DEFAULT nextval('group_message_record_read_id_seq'::regclass) NOT NULL,
    nano_id varchar NOT NULL,
    "timestamp" int8 NOT NULL,
    send_user uuid NOT NULL,
    group_uuid uuid NOT NULL,
    read_user uuid NOT NULL,
    CONSTRAINT group_message_record_read_pk PRIMARY KEY (id)
);

-- Table comment
COMMENT ON TABLE public.group_message_record_read IS '群消息已读状态表';

-- Column comments
COMMENT ON COLUMN public.group_message_record_read.id IS '自增id';
COMMENT ON COLUMN public.group_message_record_read.nano_id IS '消息主键';
COMMENT ON COLUMN public.group_message_record_read."timestamp" IS '消息创建时间';
COMMENT ON COLUMN public.group_message_record_read.send_user IS '发送人id';
COMMENT ON COLUMN public.group_message_record_read.group_uuid IS '群组id';
COMMENT ON COLUMN public.group_message_record_read.read_user IS '已读用户id';
