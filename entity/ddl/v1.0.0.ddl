-- 系统通知表
CREATE TABLE system_notification (
    -- 主键ID
    id UUID PRIMARY KEY,
    
    -- 通知标题
    title VARCHAR(255),
    
    -- 详细内容
    content TEXT,
    
    -- 创建时间 (Unix时间戳，毫秒)
    created_at BIGINT,
    
    -- 内容类型，0-纯文本，1-json, 2-xml
    content_type SMALLINT,
    
    -- 接收人
    user_id UUID,
    
    -- 是否已读
    is_read BOOLEAN,
    
    -- 第一层级，用于定位功能大类
    level1 INTEGER,
    
    -- 第二层级，用于定位子功能模块
    level2 INTEGER,
    
    -- 第三层级，用于定位具体功能组
    level3 INTEGER,
    
    -- 第四层级，用于定位详细功能项
    level4 INTEGER,
    
    -- 未读数量
    unread_count INTEGER
);

COMMENT ON TABLE system_notification IS '系统通知表';
COMMENT ON COLUMN system_notification.id IS '主键ID';
COMMENT ON COLUMN system_notification.title IS '通知标题';
COMMENT ON COLUMN system_notification.content IS '详细内容';
COMMENT ON COLUMN system_notification.created_at IS '创建时间 (Unix时间戳，毫秒)';
COMMENT ON COLUMN system_notification.content_type IS '内容类型，0-纯文本，1-json, 2-xml';
COMMENT ON COLUMN system_notification.user_id IS '接收人';
COMMENT ON COLUMN system_notification.is_read IS '是否已读';
COMMENT ON COLUMN system_notification.level1 IS '第一层级，用于定位功能大类';
COMMENT ON COLUMN system_notification.level2 IS '第二层级，用于定位子功能模块';
COMMENT ON COLUMN system_notification.level3 IS '第三层级，用于定位具体功能组';
COMMENT ON COLUMN system_notification.level4 IS '第四层级，用于定位详细功能项';
COMMENT ON COLUMN system_notification.unread_count IS '未读数量';

-- 添加索引以提高查询性能
-- 为接收人和创建时间添加索引，便于按用户和时间查询通知
CREATE INDEX IF NOT EXISTS idx_system_notification_user_id_created_at ON system_notification(user_id, created_at);

-- 为是否已读状态添加索引
CREATE INDEX IF NOT EXISTS idx_system_notification_is_read ON system_notification(is_read);