-- public.friend_request_info table definition

-- Drop table
-- DROP TABLE friend_request_info;

CREATE TABLE IF NOT EXISTS friend_request_info (
    uuid uuid NOT NULL, -- 主键
    created_at int8 NOT NULL, -- 创建时间
    updated_at int8 NOT NULL, -- 更新时间
    request_message varchar NULL, -- 加好友的文本
    add_type varchar NULL, -- 哪种方式添加
    request_user uuid NULL, -- 请求方
    accept_user uuid NULL, -- 接收方
    accept_message varchar NULL, -- 接收方信息
    accept_status int2 NULL, -- 加好友状态
    id serial4 NOT NULL, -- 自增id
    "version" int4 NULL,
    CONSTRAINT friend_request_info_pk PRIMARY KEY (id)
);

-- Table comment
COMMENT ON TABLE public.friend_request_info IS '好友请求表';

-- Column comments
COMMENT ON COLUMN public.friend_request_info.uuid IS '主键';
COMMENT ON COLUMN public.friend_request_info.created_at IS '创建时间';
COMMENT ON COLUMN public.friend_request_info.updated_at IS '更新时间';
COMMENT ON COLUMN public.friend_request_info.request_message IS '加好友的文本';
COMMENT ON COLUMN public.friend_request_info.add_type IS '哪种方式添加';
COMMENT ON COLUMN public.friend_request_info.request_user IS '请求方';
COMMENT ON COLUMN public.friend_request_info.accept_user IS '接收方';
COMMENT ON COLUMN public.friend_request_info.accept_message IS '接收方信息';
COMMENT ON COLUMN public.friend_request_info.accept_status IS '加好友状态';
COMMENT ON COLUMN public.friend_request_info.id IS '自增id';
COMMENT ON COLUMN public.friend_request_info."version" IS '版本号';
