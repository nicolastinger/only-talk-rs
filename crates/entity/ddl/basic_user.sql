-- public.basic_user table definition

-- Drop table
-- DROP TABLE basic_user;

CREATE TABLE IF NOT EXISTS basic_user (
    uuid uuid NOT NULL, -- 主键
    username varchar NOT NULL, -- 用户名
    account varchar NOT NULL, -- 用户编码
    "password" varchar NOT NULL, -- 用户密码
    info varchar NULL, -- 个人简介
    icon varchar NULL, -- 用户头像超链接
    CONSTRAINT basic_user_pk PRIMARY KEY (uuid)
);

-- Table comment
COMMENT ON TABLE public.basic_user IS '基础用户表';

-- Column comments
COMMENT ON COLUMN public.basic_user.uuid IS '主键';
COMMENT ON COLUMN public.basic_user.username IS '用户名';
COMMENT ON COLUMN public.basic_user.account IS '用户编码';
COMMENT ON COLUMN public.basic_user."password" IS '用户密码';
COMMENT ON COLUMN public.basic_user.info IS '个人简介';
COMMENT ON COLUMN public.basic_user.icon IS '用户头像超链接';
