-- public.report 表定义

-- 删除表
-- DROP TABLE report;

CREATE TABLE IF NOT EXISTS report (
    uuid uuid NOT NULL, -- 主键ID
    reporter_uuid uuid NOT NULL, -- 举报人 uuid
    target_type int2 NOT NULL, -- 举报目标类型: 1=用户, 2=群组, 3=动态, 4=卡片匹配(交友广场用户), 5=动态评论
    target_uuid uuid NOT NULL, -- 举报目标主键
    reason text NOT NULL, -- 举报原因/描述
    status int2 DEFAULT 0 NOT NULL, -- 处理状态: 0=待处理, 1=已受理, 2=已驳回
    is_del bool DEFAULT false NOT NULL, -- 是否删除
    created_at int8 NOT NULL, -- 创建时间
    updated_at int8 NOT NULL, -- 更新时间
    CONSTRAINT report_pk PRIMARY KEY (uuid)
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_report_target ON public.report USING btree (target_type, target_uuid);
CREATE INDEX IF NOT EXISTS idx_report_reporter_created ON public.report USING btree (reporter_uuid, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_report_status_created ON public.report USING btree (status, created_at DESC);

-- 表注释
COMMENT ON TABLE public.report IS '举报记录表';

-- 列注释
COMMENT ON COLUMN public.report.uuid IS '主键ID (uuid)';
COMMENT ON COLUMN public.report.reporter_uuid IS '举报人uuid';
COMMENT ON COLUMN public.report.target_type IS '举报目标类型: 1=用户, 2=群组, 3=动态, 4=卡片匹配(交友广场用户), 5=动态评论';
COMMENT ON COLUMN public.report.target_uuid IS '举报目标主键 (用户/群/动态/广场用户 uuid 或 评论 id)';
COMMENT ON COLUMN public.report.reason IS '举报原因/描述 (文字)';
COMMENT ON COLUMN public.report.status IS '处理状态: 0=待处理, 1=已受理, 2=已驳回';
COMMENT ON COLUMN public.report.is_del IS '是否删除';
COMMENT ON COLUMN public.report.created_at IS '创建时间 (Unix时间戳，秒)';
COMMENT ON COLUMN public.report.updated_at IS '更新时间 (Unix时间戳，秒)';
