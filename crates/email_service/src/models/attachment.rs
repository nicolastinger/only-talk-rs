//! 邮件附件定义
//!
//! 本模块定义了 [`Attachment`] 结构体和 [`ContentDisposition`] 枚举。

use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine};
use mime::Mime;
use serde::{Deserialize, Serialize};
use std::fmt;

/// 邮件附件
///
/// 表示邮件中的一个附件，包含文件名、内容和类型信息。
///
/// # 创建附件
///
/// ```rust
/// use email_service::models::{Attachment, ContentDisposition};
///
/// // 从字节数据创建
/// let attachment = Attachment::new(
///     "document.pdf",
///     vec![0x25, 0x50, 0x44, 0x46],
///     "application/pdf"
/// );
///
/// // 自动检测类型
/// let attachment = Attachment::from_bytes("image.png", vec![0x89, 0x50, 0x4E, 0x47]);
///
/// // 从 Base64 创建
/// let attachment = Attachment::from_base64("file.txt", "SGVsbG8gV29ybGQ=")?;
/// # Ok::<(), email_service::error::EmailError>(())
/// ```
///
/// # 内联附件
///
/// 内联附件通常用于 HTML 邮件中嵌入图片：
///
/// ```rust
/// use email_service::models::{Attachment, ContentDisposition};
///
/// let inline_image = Attachment::inline(
///     "logo.png",
///     vec![/* 图片数据 */],
///     "image/png"
/// ).with_content_id("logo");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    /// 文件名
    pub filename: String,

    /// 文件内容（二进制数据）
    pub content: Vec<u8>,

    /// MIME 类型
    ///
    /// 如 `application/pdf`、`image/png`、`text/plain` 等。
    pub content_type: String,

    /// 内容 ID（用于内联附件）
    ///
    /// 在 HTML 中通过 `cid:content_id` 引用。
    pub content_id: Option<String>,

    /// 内容处置方式
    pub disposition: ContentDisposition,
}

impl Attachment {
    /// 创建新附件
    ///
    /// # 参数
    ///
    /// - `filename`: 文件名
    /// - `content`: 文件内容
    /// - `content_type`: MIME 类型
    ///
    /// # 示例
    ///
    /// ```rust
    /// use email_service::models::Attachment;
    ///
    /// let attachment = Attachment::new(
    ///     "report.pdf",
    ///     vec![/* PDF 数据 */],
    ///     "application/pdf"
    /// );
    /// ```
    pub fn new(
        filename: impl Into<String>,
        content: Vec<u8>,
        content_type: impl Into<String>,
    ) -> Self {
        Self {
            filename: filename.into(),
            content,
            content_type: content_type.into(),
            content_id: None,
            disposition: ContentDisposition::Attachment,
        }
    }

    /// 设置内容 ID
    ///
    /// 用于内联附件，在 HTML 中通过 `cid:xxx` 引用。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use email_service::models::Attachment;
    ///
    /// let attachment = Attachment::new(
    ///     "logo.png",
    ///     vec![/* 图片数据 */],
    ///     "image/png"
    /// ).with_content_id("company-logo");
    ///
    /// // 在 HTML 中引用: <img src="cid:company-logo">
    /// ```
    pub fn with_content_id(mut self, content_id: impl Into<String>) -> Self {
        self.content_id = Some(content_id.into());
        self
    }

    /// 设置内容处置方式
    pub fn with_disposition(mut self, disposition: ContentDisposition) -> Self {
        self.disposition = disposition;
        self
    }

    /// 创建内联附件
    ///
    /// 内联附件通常用于 HTML 邮件中嵌入图片。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use email_service::models::Attachment;
    ///
    /// let logo = Attachment::inline(
    ///     "logo.png",
    ///     vec![/* PNG 数据 */],
    ///     "image/png"
    /// );
    /// ```
    pub fn inline(filename: impl Into<String>, content: Vec<u8>, content_type: impl Into<String>) -> Self {
        Self {
            filename: filename.into(),
            content,
            content_type: content_type.into(),
            content_id: None,
            disposition: ContentDisposition::Inline,
        }
    }

    /// 从字节数据创建附件（自动检测类型）
    ///
    /// 根据文件扩展名和文件头自动检测 MIME 类型。
    ///
    /// # 支持的类型
    ///
    /// | 扩展名 | MIME 类型 |
    /// |--------|-----------|
    /// | .pdf | application/pdf |
    /// | .doc | application/msword |
    /// | .docx | application/vnd.openxmlformats-... |
    /// | .png | image/png |
    /// | .jpg, .jpeg | image/jpeg |
    /// | .gif | image/gif |
    /// | .txt | text/plain |
    /// | .html | text/html |
    /// | .json | application/json |
    /// | .zip | application/zip |
    ///
    /// # 示例
    ///
    /// ```rust
    /// use email_service::models::Attachment;
    ///
    /// let attachment = Attachment::from_bytes("report.pdf", vec![/* 数据 */]);
    /// assert_eq!(attachment.content_type, "application/pdf");
    /// ```
    pub fn from_bytes(
        filename: impl Into<String>,
        content: Vec<u8>,
    ) -> Self {
        let filename_str = filename.into();
        let content_type = Self::detect_content_type(&content, &filename_str);
        Self::new(filename_str, content, content_type)
    }

    /// 从 Base64 字符串创建附件
    ///
    /// # 参数
    ///
    /// - `filename`: 文件名
    /// - `base64_content`: Base64 编码的文件内容
    ///
    /// # 错误
    ///
    /// 如果 Base64 解码失败，返回 [`EmailError::AttachmentError`]。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use email_service::models::Attachment;
    ///
    /// let attachment = Attachment::from_base64(
    ///     "hello.txt",
    ///     "SGVsbG8gV29ybGQ="
    /// )?;
    /// # Ok::<(), email_service::error::EmailError>(())
    /// ```
    pub fn from_base64(
        filename: impl Into<String>,
        base64_content: &str,
    ) -> crate::error::EmailResult<Self> {
        let content = BASE64_STANDARD
            .decode(base64_content)
            .map_err(|e| crate::error::EmailError::AttachmentError(format!("Failed to decode base64: {}", e)))?;
        Ok(Self::from_bytes(filename, content))
    }

    /// 获取附件大小（字节）
    pub fn size(&self) -> usize {
        self.content.len()
    }

    /// 解析 MIME 类型
    ///
    /// # 错误
    ///
    /// 如果 MIME 类型格式无效，返回解析错误。
    pub fn mime_type(&self) -> Result<Mime, mime::FromStrError> {
        self.content_type.parse()
    }

    /// 检测文件类型
    ///
    /// 根据文件扩展名和文件头（魔数）检测 MIME 类型。
    fn detect_content_type(content: &[u8], filename: &str) -> String {
        if let Some(extension) = filename.rsplit('.').next() {
            match extension.to_lowercase().as_str() {
                "pdf" => return "application/pdf".to_string(),
                "doc" => return "application/msword".to_string(),
                "docx" => return "application/vnd.openxmlformats-officedocument.wordprocessingml.document".to_string(),
                "xls" => return "application/vnd.ms-excel".to_string(),
                "xlsx" => return "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".to_string(),
                "png" => return "image/png".to_string(),
                "jpg" | "jpeg" => return "image/jpeg".to_string(),
                "gif" => return "image/gif".to_string(),
                "txt" => return "text/plain".to_string(),
                "html" | "htm" => return "text/html".to_string(),
                "zip" => return "application/zip".to_string(),
                "json" => return "application/json".to_string(),
                _ => {}
            }
        }

        if content.starts_with(b"%PDF") {
            return "application/pdf".to_string();
        }
        if content.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
            return "image/png".to_string();
        }
        if content.starts_with(&[0xFF, 0xD8, 0xFF]) {
            return "image/jpeg".to_string();
        }

        "application/octet-stream".to_string()
    }
}

/// 内容处置方式
///
/// 定义附件在邮件中的显示方式。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ContentDisposition {
    /// 普通附件
    ///
    /// 作为独立文件显示，需要用户点击下载或打开。
    Attachment,

    /// 内联附件
    ///
    /// 直接嵌入邮件正文中显示，常用于 HTML 邮件中的图片。
    Inline,
}

impl Default for ContentDisposition {
    fn default() -> Self {
        Self::Attachment
    }
}

impl fmt::Display for ContentDisposition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContentDisposition::Attachment => write!(f, "attachment"),
            ContentDisposition::Inline => write!(f, "inline"),
        }
    }
}
