-- public.user_session 表定义

-- 删除表
-- DROP TABLE user_session;

CREATE TABLE IF NOT EXISTS user_session (
    id           bigserial    NOT NULL,
    user_uuid    uuid         NOT NULL,           -- 谁的列表
    session_uuid uuid         NOT NULL,           -- 关联 session
    session_type int2         NOT NULL,           -- 冗余, 免 join
    peer_uuid    uuid         NULL,               -- 单聊: 对方; 群聊: NULL
    last_read_id int8         NOT NULL DEFAULT 0, -- 已读游标(角标口径)
    synced_id    int8         NOT NULL DEFAULT 0, -- 同步游标(拉取起点), 恒 >= last_read_id
    pinned       int2         NOT NULL DEFAULT 0,
    muted        int2         NOT NULL DEFAULT 0,
    deleted_at   int8         NULL,               -- 用户软删时刻(毫秒)
    created_at   int8         NOT NULL,
    updated_at   int8         NOT NULL,
    CONSTRAINT user_session_pk PRIMARY KEY (id),
    CONSTRAINT user_session_uk UNIQUE (user_uuid, session_uuid)
);

CREATE INDEX IF NOT EXISTS idx_user_session_list
    ON public.user_session (user_uuid, pinned DESC, deleted_at);

COMMENT ON TABLE  public.user_session               IS '用户视角的会话状态：每用户每会话一行';
COMMENT ON COLUMN public.user_session.user_uuid     IS '用户id(谁的会话列表)';
COMMENT ON COLUMN public.user_session.session_uuid  IS '会话id, 关联 session 表';
COMMENT ON COLUMN public.user_session.session_type  IS '冗余会话类型, 免 join: 1-单聊 2-群聊 3-系统 4-公众号';
COMMENT ON COLUMN public.user_session.peer_uuid     IS '单聊: 对方uuid; 群聊: NULL';
COMMENT ON COLUMN public.user_session.last_read_id  IS '已读游标: 只前进不回退, 决定未读角标; 上报时钳制到 synced_id';
COMMENT ON COLUMN public.user_session.synced_id     IS '同步游标: 客户端已拉取到的位置, 拉取起点; 恒 >= last_read_id';
COMMENT ON COLUMN public.user_session.pinned        IS '置顶: 0-否 1-是';
COMMENT ON COLUMN public.user_session.muted         IS '免打扰: 0-否 1-是';
COMMENT ON COLUMN public.user_session.deleted_at    IS '软删时刻(毫秒); session.last_message_at > 此值即视为会话复活';