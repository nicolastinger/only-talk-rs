use rbatis::rbdc::Uuid;
use serde::{Deserialize, Serialize};
use entity::models::file_entity::biz_record::BizRecord;
use entity::models::file_entity::chat_biz_record::ChatBizRecord;

/// 文件上传业务表
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BizRecordVO {
    /// 业务唯一标识符
    pub uuid: Option<Uuid>,
    /// 业务名称
    pub biz_name: Option<String>,
    /// 业务描述
    pub description: Option<String>,
    /// 关联的文件UUID
    pub file_ids: Option<String>,
    /// 关联的压缩后的文件UUID
    pub preview_file_ids: Option<String>,
    /// 业务类型(头像、用户背景、广场图片等)
    pub biz_type: Option<String>,
    /// 备注信息
    pub remark: Option<String>,
}

impl BizRecordVO {
    pub fn from_biz_record(biz_record: BizRecord) -> Self {
        BizRecordVO {
            uuid: biz_record.uuid,
            biz_name: biz_record.biz_name,
            description: biz_record.description,
            file_ids: biz_record.file_ids.clone(),
            preview_file_ids: biz_record.preview_file_ids,
            biz_type: biz_record.biz_type,
            remark: biz_record.remark,
        }
    }

    pub fn from_chat_biz_record(chat_biz_record: ChatBizRecord) -> Self {
        BizRecordVO {
            uuid: chat_biz_record.uuid,
            biz_name: chat_biz_record.biz_name,
            description: chat_biz_record.description,
            file_ids: chat_biz_record.file_ids,
            preview_file_ids: chat_biz_record.preview_file_ids,
            biz_type: chat_biz_record.biz_type,
            remark: chat_biz_record.remark,
        }
    }
}