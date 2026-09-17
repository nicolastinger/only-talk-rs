-- 群组表
CREATE TABLE IF NOT EXISTS group_info (
    id BIGSERIAL PRIMARY KEY,
    group_uuid UUID NOT NULL UNIQUE,
    group_name VARCHAR(100) NOT NULL,
    avatar VARCHAR(500),
    owner_uuid UUID NOT NULL,
    description VARCHAR(500),
    max_members INTEGER DEFAULT 500,
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL,
    status SMALLINT DEFAULT 1
);

COMMENT ON TABLE group_info IS '群组信息表';
COMMENT ON COLUMN group_info.id IS '主键ID';
COMMENT ON COLUMN group_info.group_uuid IS '群UUID';
COMMENT ON COLUMN group_info.group_name IS '群名称';
COMMENT ON COLUMN group_info.avatar IS '群头像URL';
COMMENT ON COLUMN group_info.owner_uuid IS '群主UUID';
COMMENT ON COLUMN group_info.description IS '群描述';
COMMENT ON COLUMN group_info.max_members IS '最大成员数';
COMMENT ON COLUMN group_info.created_at IS '创建时间(毫秒时间戳)';
COMMENT ON COLUMN group_info.updated_at IS '更新时间(毫秒时间戳)';
COMMENT ON COLUMN group_info.status IS '群状态: 1-正常, 2-解散';

-- 群成员表
CREATE TABLE IF NOT EXISTS group_member (
    id BIGSERIAL PRIMARY KEY,
    group_uuid UUID NOT NULL REFERENCES group_info(group_uuid),
    user_uuid UUID NOT NULL,
    role SMALLINT DEFAULT 0,
    nickname VARCHAR(50),
    join_time BIGINT NOT NULL,
    muted BOOLEAN DEFAULT FALSE,
    status SMALLINT DEFAULT 1,
    UNIQUE(group_uuid, user_uuid)
);

COMMENT ON TABLE group_member IS '群成员表';
COMMENT ON COLUMN group_member.id IS '主键ID';
COMMENT ON COLUMN group_member.group_uuid IS '群UUID';
COMMENT ON COLUMN group_member.user_uuid IS '用户UUID';
COMMENT ON COLUMN group_member.role IS '角色: 0-成员, 1-管理员, 2-群主';
COMMENT ON COLUMN group_member.nickname IS '群内昵称';
COMMENT ON COLUMN group_member.join_time IS '入群时间(毫秒时间戳)';
COMMENT ON COLUMN group_member.muted IS '是否禁言';
COMMENT ON COLUMN group_member.status IS '成员状态: 1-正常, 2-已退出, 3-踢出';

-- 群消息表（读扩散，只存 1 份）—— 改为按群哈希分区
CREATE TABLE IF NOT EXISTS group_message_record (
    -- 分区表约束必须含分区键
    id BIGSERIAL,
    nano_id VARCHAR(64) NOT NULL,
    group_uuid UUID NOT NULL REFERENCES group_info(group_uuid),
    send_user UUID NOT NULL,
    timestamp BIGINT NOT NULL,
    raw BYTEA NOT NULL,
    msg_type SMALLINT DEFAULT 1,
    recalled BOOLEAN DEFAULT FALSE,
    PRIMARY KEY (id, group_uuid),
    UNIQUE (nano_id, group_uuid)
) PARTITION BY HASH (group_uuid);

-- 16 个哈希分区
DO $$
BEGIN
    FOR i IN 0..15 LOOP
        EXECUTE format(
            'CREATE TABLE IF NOT EXISTS group_message_record_p%s
             PARTITION OF group_message_record
             FOR VALUES WITH (MODULUS 16, REMAINDER %s)',
            lpad(i::text, 2, '0'), i);
    END LOOP;
END $$;

COMMENT ON TABLE group_message_record IS '群消息记录表(读扩散, 按群哈希分区)';
COMMENT ON COLUMN group_message_record.id IS '主键ID';
COMMENT ON COLUMN group_message_record.nano_id IS '消息唯一标识(nanoid, 群内唯一)';
COMMENT ON COLUMN group_message_record.group_uuid IS '群UUID';
COMMENT ON COLUMN group_message_record.send_user IS '发送者UUID';
COMMENT ON COLUMN group_message_record.timestamp IS '时间戳(毫秒)';
COMMENT ON COLUMN group_message_record.raw IS '原始消息内容';
COMMENT ON COLUMN group_message_record.msg_type IS '消息类型: 1-文本, 2-图片, 3-文件';
COMMENT ON COLUMN group_message_record.recalled IS '是否撤回';

-- 索引
CREATE INDEX IF NOT EXISTS idx_group_member_group ON group_member(group_uuid);
CREATE INDEX IF NOT EXISTS idx_group_member_user ON group_member(user_uuid);
CREATE INDEX IF NOT EXISTS idx_group_member_status ON group_member(status);
-- 与单聊对称的拉取索引(游标+排序同列, 7天窗口免回表)
CREATE INDEX IF NOT EXISTS idx_group_msg_pull ON group_message_record (group_uuid, id) INCLUDE ("timestamp");
-- 保留原索引(会话内按时间翻历史, offset 分页)
CREATE INDEX IF NOT EXISTS idx_group_msg_group_time ON group_message_record(group_uuid, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_group_msg_send_user ON group_message_record(send_user);
