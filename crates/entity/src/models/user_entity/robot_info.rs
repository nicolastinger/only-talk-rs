use rbatis::crud;
use rbatis::executor::Executor;
use rbatis::rbdc::Uuid;
use serde::{Deserialize, Serialize};

/// 机器人扩展信息表 (逻辑关联 basic_user.uuid, user_type=1)
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct RobotInfo {
    /// 机器人唯一标识符 (UUID, 逻辑关联 basic_user.uuid)
    pub uuid: Option<Uuid>,
    /// 归属用户 UUID (谁创建的机器人, 逻辑关联 basic_user.uuid)
    pub owner_uuid: Option<Uuid>,
    /// 模型标识 (如 gpt-4)
    pub model: Option<String>,
    /// 人设/系统提示词
    pub prompt: Option<String>,
    /// 是否启用 (0: 禁用, 1: 启用)
    pub enabled: Option<i16>,
    /// 状态 (0: 正常, 1: 禁用, 2: 注销等)
    pub status: Option<i16>,
    /// 备注
    pub note: Option<String>,
    /// 记录创建时间 (Unix 时间戳，单位：秒)
    pub created_at: Option<i64>,
    /// 记录最后更新时间 (Unix 时间戳，单位：秒)
    pub updated_at: Option<i64>,
}

crud!(RobotInfo {});

impl RobotInfo {
    #[rbatis::py_sql("select * from robot_info where uuid = #{uuid} limit 1")]
    async fn select_by_uuid_inner(rb: &dyn Executor, uuid: &Uuid) -> Vec<RobotInfo> {}

    pub async fn select_by_uuid(
        rb: &dyn Executor,
        uuid: &Uuid,
    ) -> rbatis::Result<Option<RobotInfo>> {
        Ok(Self::select_by_uuid_inner(rb, uuid).await?.into_iter().next())
    }

    pub async fn update_by_uuid(
        rb: &dyn Executor,
        table: &RobotInfo,
        uuid: &Uuid,
    ) -> Result<rbatis::rbdc::db::ExecResult, rbatis::rbdc::Error> {
        RobotInfo::update_by_map(rb, table, rbs::value! {"uuid": uuid}).await
    }
}
