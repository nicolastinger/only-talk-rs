use crate::http_service::file_service::vo::biz_file_link_vo::BizFileLinkVO;
use common::models::file_entity::biz_record::BizRecord;
use common::models::file_entity::chat_biz_record::ChatBizRecord;
use rbatis::rbdc::Uuid;
use serde::{Deserialize, Serialize};

/// 文件上传业务表
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BizRecordVO {
    /// 业务唯一标识符
    pub uuid: Option<Uuid>,
    /// 业务名称
    pub biz_name: Option<String>,
    /// 业务描述
    pub description: Option<String>,
    /// 业务类型(头像、用户背景、广场图片等)
    pub biz_type: Option<String>,
    /// 备注信息
    pub remark: Option<String>,
    /// 关联的文件信息
    pub file_infos: Option<Vec<BizFileLinkVO>>,
}

impl BizRecordVO {
    pub fn from_biz_record(biz_record: BizRecord, file_ids: Vec<BizFileLinkVO>) -> Self {
        BizRecordVO {
            uuid: biz_record.uuid,
            biz_name: biz_record.biz_name,
            description: biz_record.description,
            biz_type: biz_record.biz_type,
            remark: biz_record.remark,
            file_infos: Some(file_ids),
        }
    }

    pub fn from_chat_biz_record(
        chat_biz_record: ChatBizRecord,
        file_ids: Vec<BizFileLinkVO>,
    ) -> Self {
        BizRecordVO {
            uuid: chat_biz_record.uuid,
            biz_name: chat_biz_record.biz_name,
            description: chat_biz_record.description,
            biz_type: chat_biz_record.biz_type,
            remark: chat_biz_record.remark,
            file_infos: Some(file_ids),
        }
    }
}
