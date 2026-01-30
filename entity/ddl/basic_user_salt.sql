-- public.basic_user_salt table definition

-- Drop table
-- DROP TABLE basic_user_salt;

CREATE TABLE IF NOT EXISTS basic_user_salt (
    uuid uuid NOT NULL, -- 主键
    sign_up_salt varchar NULL, -- 密码盐
    CONSTRAINT basic_user_salt_pk PRIMARY KEY (uuid)
);

-- Table comment
COMMENT ON TABLE public.basic_user_salt IS '用户密码盐表';

-- Column comments
COMMENT ON COLUMN public.basic_user_salt.uuid IS '主键';
COMMENT ON COLUMN public.basic_user_salt.sign_up_salt IS '密码盐';
