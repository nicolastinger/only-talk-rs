use rbatis::executor::Executor;
use rbatis::rbdc::Uuid;
use rbatis::crud;
use serde::{Deserialize, Serialize};

/// 公开文件业务表关联
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BizFileLink {
    /// 主键ID
    pub id: Option<i64>,
    /// 业务唯一标识符
    pub biz_id: Option<Uuid>,
    /// 原文件唯一标识符
    pub origin_file_id: Option<Uuid>,
    /// 预览文件唯一标识符
    pub file_id: Option<Uuid>,
    /// 是否删除
    pub is_del: Option<bool>,
}

crud!(BizFileLink {});

impl BizFileLink {
    #[rbatis::py_sql("select * from biz_file_link where biz_id = #{biz_id} and (file_id = #{file_id} or origin_file_id = #{file_id})")]
    async fn select_by_biz_and_file_inner(rb: &dyn Executor, biz_id: &Uuid, file_id: &Uuid) -> Vec<BizFileLink> {}

    pub async fn select_by_biz_and_file(
        rb: &dyn Executor,
        biz_id: &Uuid,
        file_id: &Uuid,
    ) -> rbatis::Result<Option<BizFileLink>> {
        Ok(Self::select_by_biz_and_file_inner(rb, biz_id, file_id).await?.into_iter().next())
    }

    #[rbatis::py_sql("select * from biz_file_link where biz_id = #{biz_id} limit 100")]
    async fn select_by_biz(rb: &dyn Executor, biz_id: &Uuid) -> Vec<BizFileLink> {}
}
