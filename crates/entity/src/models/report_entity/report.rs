use rbatis::crud;
use rbatis::executor::Executor;
use rbatis::rbdc::Uuid;
use serde::{Deserialize, Serialize};

/// 举报目标类型: 1=用户, 2=群组, 3=动态, 4=卡片匹配(交友广场用户), 5=动态评论
pub const TARGET_USER: i16 = 1;
pub const TARGET_GROUP: i16 = 2;
pub const TARGET_MOMENT: i16 = 3;
pub const TARGET_PLAZA_USER: i16 = 4;
pub const TARGET_MOMENT_COMMENT: i16 = 5;

/// 举报处理状态: 0=待处理, 1=已受理, 2=已驳回
pub const STATUS_PENDING: i16 = 0;
pub const STATUS_ACCEPTED: i16 = 1;
pub const STATUS_REJECTED: i16 = 2;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Report {
    /// 主键ID
    pub uuid: Option<Uuid>,
    /// 举报人 uuid
    pub reporter_uuid: Option<Uuid>,
    /// 举报目标类型: 1=用户, 2=群组, 3=动态, 4=卡片匹配(交友广场用户), 5=动态评论
    pub target_type: Option<i16>,
    /// 举报目标主键 (用户/群/动态/广场用户 uuid 或 评论 id)
    pub target_uuid: Option<Uuid>,
    /// 举报原因/描述 (文字)
    pub reason: Option<String>,
    /// 处理状态: 0=待处理, 1=已受理, 2=已驳回
    pub status: Option<i16>,
    /// 是否删除
    pub is_del: Option<bool>,
    /// 创建时间 (Unix 时间戳，秒)
    pub created_at: Option<i64>,
    /// 更新时间 (Unix 时间戳，秒)
    pub updated_at: Option<i64>,
}

crud!(Report {});

impl Report {
    #[rbatis::py_sql("select * from report where uuid = #{uuid} limit 1")]
    async fn select_by_uuid_inner(rb: &dyn Executor, uuid: &Uuid) -> Vec<Report> {}

    pub async fn select_by_uuid(rb: &dyn Executor, uuid: &Uuid) -> rbatis::Result<Option<Report>> {
        Ok(Self::select_by_uuid_inner(rb, uuid).await?.into_iter().next())
    }

    /// 查询举报人对同一目标的待处理举报(用于去重)
    #[rbatis::py_sql(
        "select * from report where reporter_uuid = #{reporter_uuid} and target_type = #{target_type} and target_uuid = #{target_uuid} and status = 0 and is_del = false limit 1"
    )]
    async fn select_pending_by_reporter_target_inner(
        rb: &dyn Executor,
        reporter_uuid: &Uuid,
        target_type: i16,
        target_uuid: &Uuid,
    ) -> Vec<Report> {
    }

    pub async fn select_pending_by_reporter_target(
        rb: &dyn Executor,
        reporter_uuid: &Uuid,
        target_type: i16,
        target_uuid: &Uuid,
    ) -> rbatis::Result<Option<Report>> {
        Ok(Self::select_pending_by_reporter_target_inner(
            rb,
            reporter_uuid,
            target_type,
            target_uuid,
        )
        .await?
        .into_iter()
        .next())
    }
}
