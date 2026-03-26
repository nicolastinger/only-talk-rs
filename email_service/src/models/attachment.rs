use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine};
use mime::Mime;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub filename: String,
    pub content: Vec<u8>,
    pub content_type: String,
    pub content_id: Option<String>,
    pub disposition: ContentDisposition,
}

impl Attachment {
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

    pub fn with_content_id(mut self, content_id: impl Into<String>) -> Self {
        self.content_id = Some(content_id.into());
        self
    }

    pub fn with_disposition(mut self, disposition: ContentDisposition) -> Self {
        self.disposition = disposition;
        self
    }

    pub fn inline(filename: impl Into<String>, content: Vec<u8>, content_type: impl Into<String>) -> Self {
        Self {
            filename: filename.into(),
            content,
            content_type: content_type.into(),
            content_id: None,
            disposition: ContentDisposition::Inline,
        }
    }

    pub fn from_bytes(
        filename: impl Into<String>,
        content: Vec<u8>,
    ) -> Self {
        let filename_str = filename.into();
        let content_type = Self::detect_content_type(&content, &filename_str);
        Self::new(filename_str, content, content_type)
    }

    pub fn from_base64(
        filename: impl Into<String>,
        base64_content: &str,
    ) -> crate::error::EmailResult<Self> {
        let content = BASE64_STANDARD
            .decode(base64_content)
            .map_err(|e| crate::error::EmailError::AttachmentError(format!("Failed to decode base64: {}", e)))?;
        Ok(Self::from_bytes(filename, content))
    }

    pub fn size(&self) -> usize {
        self.content.len()
    }

    pub fn mime_type(&self) -> Result<Mime, mime::FromStrError> {
        self.content_type.parse()
    }

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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ContentDisposition {
    Attachment,
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