-- public.user_info table definition

-- Drop table
-- DROP TABLE user_info;

CREATE TABLE IF NOT EXISTS user_info (
    uuid uuid NOT NULL, -- 主键
    gender int4 DEFAULT 0 NOT NULL, -- 性别
    age int2 NULL, -- 年龄
    birthday int8 DEFAULT 0 NOT NULL,
    created_at int8 NULL, -- 创建时间
    updated_at int8 NULL, -- 更新时间
    phone varchar NULL, -- 手机号
    email varchar NULL, -- 邮箱
    address varchar NULL, -- 地址
    status int2 NULL, -- 状态
    note varchar NULL, -- 用户简介
    CONSTRAINT user_info_pk PRIMARY KEY (uuid)
);

-- Table comment
COMMENT ON TABLE public.user_info IS '用户详细信息表';

-- Column comments
COMMENT ON COLUMN public.user_info.uuid IS '主键';
COMMENT ON COLUMN public.user_info.gender IS '性别';
COMMENT ON COLUMN public.user_info.age IS '年龄';
COMMENT ON COLUMN public.user_info.birthday IS '生日';
COMMENT ON COLUMN public.user_info.created_at IS '创建时间';
COMMENT ON COLUMN public.user_info.updated_at IS '更新时间';
COMMENT ON COLUMN public.user_info.phone IS '手机号';
COMMENT ON COLUMN public.user_info.email IS '邮箱';
COMMENT ON COLUMN public.user_info.address IS '地址';
COMMENT ON COLUMN public.user_info.status IS '状态';
COMMENT ON COLUMN public.user_info.note IS '用户简介';
