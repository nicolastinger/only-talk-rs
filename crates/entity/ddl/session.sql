-- public.session 表定义

-- 删除表
-- DROP TABLE session;

CREATE TABLE IF NOT EXISTS session (
    session_uuid    uuid          NOT NULL,   -- 会话标识: 单聊由用户对 v5 派生, 群聊即 group_uuid
    session_type    int2          NOT NULL,   -- 1-单聊 2-群聊 3-系统 4-公众号
    last_message_id int8          NULL,       -- 该会话最新消息 id(由后台聚合任务维护, 非实时)
    last_message_at int8          NULL,       -- 该会话最新消息时间/毫秒(由后台聚合任务维护, 非实时)
    last_preview    varchar(256)  NULL,       -- 列表摘要, 已截断(由后台聚合任务维护, 非实时)
    created_at      int8          NOT NULL,
    updated_at      int8          NOT NULL,
    CONSTRAINT session_pk PRIMARY KEY (session_uuid)
);

COMMENT ON TABLE  public.session                 IS '会话本体：单聊=用户对，群聊=群，双方/全员共享一行';
COMMENT ON COLUMN public.session.session_uuid    IS '会话标识: 单聊由用户对 v5 派生, 群聊即 group_uuid';
COMMENT ON COLUMN public.session.session_type    IS '1-单聊 2-群聊 3-系统 4-公众号';
COMMENT ON COLUMN public.session.last_message_id IS '最后一条消息 id, 由后台聚合任务维护, 非实时更新, 只前进不回退';
COMMENT ON COLUMN public.session.last_message_at IS '最后一条消息时间(毫秒), 由后台聚合任务维护, 非实时更新, 用于会话列表排序';
COMMENT ON COLUMN public.session.last_preview    IS '会话列表摘要, 已截断至 256 字符, 由后台聚合任务维护, 非实时更新';