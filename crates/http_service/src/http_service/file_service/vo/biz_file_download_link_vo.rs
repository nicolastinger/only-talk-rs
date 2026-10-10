use serde::{Deserialize, Serialize};

/// 文件下载链接(含原始文件名/MIME, 供客户端直接确定文件名与扩展名)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BizFileDownloadLinkVO {
    /// 下载 URL(公开桶为直链; 其他为预签名 URL)
    pub url: String,
    /// 原始文件名(含扩展名)
    pub file_name: String,
    /// MIME 类型(可能为空)
    pub mime_type: Option<String>,
}
