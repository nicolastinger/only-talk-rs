-- public.chat_list_link table definition

-- Drop table
-- DROP TABLE chat_list_link;

CREATE TABLE IF NOT EXISTS chat_list_link (
    uuid uuid NOT NULL, -- 用户id
    friend_uuid uuid NOT NULL, -- 朋友id
    "enable" int2 DEFAULT 0 NOT NULL, -- 是否显示
    created_at int8 NULL, -- 创建时间
    id serial4 NOT NULL, -- 自增主键
    CONSTRAINT chat_list_link_pk PRIMARY KEY (id),
    CONSTRAINT chat_list_link_unique UNIQUE (id)
);

-- Table comment
COMMENT ON TABLE public.chat_list_link IS '聊天列表';

-- Column comments
COMMENT ON COLUMN public.chat_list_link.uuid IS '用户id';
COMMENT ON COLUMN public.chat_list_link.friend_uuid IS '朋友id';
COMMENT ON COLUMN public.chat_list_link."enable" IS '是否显示';
COMMENT ON COLUMN public.chat_list_link.created_at IS '创建时间';
COMMENT ON COLUMN public.chat_list_link.id IS '自增主键';
