-- 群邀请表
CREATE TABLE IF NOT EXISTS group_invitation (
    id BIGSERIAL PRIMARY KEY,
    group_uuid UUID NOT NULL REFERENCES group_info(group_uuid),
    inviter_uuid UUID NOT NULL,
    invitee_uuid UUID NOT NULL,
    status SMALLINT DEFAULT 1,
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL
);

COMMENT ON TABLE group_invitation IS '群邀请记录表';
COMMENT ON COLUMN group_invitation.id IS '主键ID';
COMMENT ON COLUMN group_invitation.group_uuid IS '群UUID';
COMMENT ON COLUMN group_invitation.inviter_uuid IS '邀请人UUID';
COMMENT ON COLUMN group_invitation.invitee_uuid IS '被邀请人UUID';
COMMENT ON COLUMN group_invitation.status IS '邀请状态: 1-待处理, 2-已接受, 3-已拒绝';
COMMENT ON COLUMN group_invitation.created_at IS '创建时间(毫秒时间戳)';
COMMENT ON COLUMN group_invitation.updated_at IS '更新时间(毫秒时间戳)';

CREATE INDEX IF NOT EXISTS idx_group_invitation_invitee ON group_invitation(invitee_uuid);
CREATE INDEX IF NOT EXISTS idx_group_invitation_group ON group_invitation(group_uuid);
CREATE UNIQUE INDEX IF NOT EXISTS idx_group_invitation_pending ON group_invitation(group_uuid, invitee_uuid) WHERE status = 1;
