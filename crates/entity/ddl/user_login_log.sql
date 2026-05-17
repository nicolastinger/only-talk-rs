-- public.user_login_log table definition

-- Drop table
-- DROP TABLE user_login_log;

CREATE TABLE IF NOT EXISTS user_login_log (
    id bigserial PRIMARY KEY, -- 主键ID
    last_login_at int8, -- 最后登录时间（Unix时间戳，毫秒）
    last_login_equipment varchar(255), -- 最后登录的设备
    last_login_ipv4 varchar(45), -- 最后登录的IPv4地址
    last_login_ipv6 varchar(128) -- 最后登录的IPv6地址
);

-- Table comment
COMMENT ON TABLE public.user_login_log IS '用户登录记录表';

-- Column comments
COMMENT ON COLUMN public.user_login_log.id IS '主键ID';
COMMENT ON COLUMN public.user_login_log.last_login_at IS '最后登录时间（Unix时间戳，毫秒）';
COMMENT ON COLUMN public.user_login_log.last_login_equipment IS '最后登录的设备';
COMMENT ON COLUMN public.user_login_log.last_login_ipv4 IS '最后登录的IPv4地址';
COMMENT ON COLUMN public.user_login_log.last_login_ipv6 IS '最后登录的IPv6地址';
