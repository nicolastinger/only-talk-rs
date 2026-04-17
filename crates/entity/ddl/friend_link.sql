-- public.friend_link table definition

-- Drop table
-- DROP TABLE friend_link;

CREATE TABLE IF NOT EXISTS friend_link (
    uuid uuid NOT NULL,
    request_user uuid NOT NULL,
    accept_user uuid NOT NULL,
    is_del bool DEFAULT false NOT NULL,
    created_at int8 NOT NULL, -- 创建时间
    updated_at int8 NOT NULL, -- 更新时间
    "version" int4 DEFAULT 0 NOT NULL,
    CONSTRAINT friend_link_pk PRIMARY KEY (uuid),
    CONSTRAINT friend_link_unique UNIQUE (request_user, accept_user)
);

-- Table comment
COMMENT ON TABLE public.friend_link IS '好友关系表';

-- Column comments
COMMENT ON COLUMN public.friend_link.uuid IS '主键';
COMMENT ON COLUMN public.friend_link.request_user IS '请求用户';
COMMENT ON COLUMN public.friend_link.accept_user IS '接受用户';
COMMENT ON COLUMN public.friend_link.is_del IS '是否删除';
COMMENT ON COLUMN public.friend_link.created_at IS '创建时间';
COMMENT ON COLUMN public.friend_link.updated_at IS '更新时间';
COMMENT ON COLUMN public.friend_link."version" IS '版本号';
