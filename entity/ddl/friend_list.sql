-- public.friend_list table definition

-- Drop table
-- DROP TABLE friend_list;

CREATE TABLE IF NOT EXISTS friend_list (
    uuid uuid NOT NULL, -- 用户id
    create_at int8 NULL, -- 创建时间
    update_at int8 NULL, -- 更新时间
    "version" uuid NULL, -- 版本id，缓存对比
    last_query int8 NULL, -- 最后一次查询时间
    CONSTRAINT friend_list_pk PRIMARY KEY (uuid)
);

-- Table comment
COMMENT ON TABLE public.friend_list IS '好友列表缓存表';

-- Column comments
COMMENT ON COLUMN public.friend_list.uuid IS '用户id';
COMMENT ON COLUMN public.friend_list.create_at IS '创建时间';
COMMENT ON COLUMN public.friend_list.update_at IS '更新时间';
COMMENT ON COLUMN public.friend_list."version" IS '版本id，缓存对比';
COMMENT ON COLUMN public.friend_list.last_query IS '最后一次查询时间';
