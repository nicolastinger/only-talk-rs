-- public.user_login_log 表定义
-- 登录审计流水表: 记录每次登录/刷新 token 事件(成功/失败/账号不存在),用于安全审计与风控

-- 删除表
-- DROP TABLE user_login_log;

CREATE TABLE IF NOT EXISTS user_login_log (
    id bigserial PRIMARY KEY, -- 主键ID
    uuid uuid NULL, -- 关联 basic_user.uuid(账号不存在时为空,用户删除后置空保留审计)
    account varchar(255) NOT NULL, -- 登录时提交的账号(失败/不存在也保留,便于审计)
    login_type varchar(50) NOT NULL, -- 登录渠道: account / email(预留) / refresh
    event_type varchar(50) NOT NULL, -- 事件类型: success / password_fail / account_not_found / refresh
    login_at int8 NOT NULL, -- 事件时间(Unix时间戳,毫秒)
    platform varchar(50) NULL, -- 登录平台: PC / MOBILE
    ipv4 varchar(45) NULL, -- 客户端IPv4地址
    ipv6 varchar(128) NULL, -- 客户端IPv6地址
    user_agent varchar(512) NULL, -- 客户端 User-Agent(审计常用)
    device varchar(255) NULL, -- 设备指纹/名称(预留)
    result varchar(255) NULL, -- 结果补充(如刷新失败原因)
    CONSTRAINT user_login_log_uuid_fk FOREIGN KEY (uuid) REFERENCES basic_user (uuid) ON DELETE SET NULL
);

-- 表注释
COMMENT ON TABLE public.user_login_log IS '用户登录审计流水表(记录登录/刷新事件,用于安全审计与风控)';

-- 列注释
COMMENT ON COLUMN public.user_login_log.id IS '主键ID';
COMMENT ON COLUMN public.user_login_log.uuid IS '关联 basic_user.uuid(账号不存在时为空,用户删除后置空保留审计)';
COMMENT ON COLUMN public.user_login_log.account IS '登录时提交的账号(失败/不存在也保留,便于审计)';
COMMENT ON COLUMN public.user_login_log.login_type IS '登录渠道: account / email(预留) / refresh';
COMMENT ON COLUMN public.user_login_log.event_type IS '事件类型: success / password_fail / account_not_found / refresh';
COMMENT ON COLUMN public.user_login_log.login_at IS '事件时间(Unix时间戳,毫秒)';
COMMENT ON COLUMN public.user_login_log.platform IS '登录平台: PC / MOBILE';
COMMENT ON COLUMN public.user_login_log.ipv4 IS '客户端IPv4地址';
COMMENT ON COLUMN public.user_login_log.ipv6 IS '客户端IPv6地址';
COMMENT ON COLUMN public.user_login_log.user_agent IS '客户端 User-Agent(审计常用)';
COMMENT ON COLUMN public.user_login_log.device IS '设备指纹/名称(预留)';
COMMENT ON COLUMN public.user_login_log.result IS '结果补充(如刷新失败原因)';

-- 审计查询索引
CREATE INDEX IF NOT EXISTS idx_user_login_log_uuid_at ON user_login_log (uuid, login_at DESC);
CREATE INDEX IF NOT EXISTS idx_user_login_log_account_at ON user_login_log (account, login_at DESC);
CREATE INDEX IF NOT EXISTS idx_user_login_log_at ON user_login_log (login_at DESC);
