use anyhow::Result;
use common::read_global_array_config;

/// 文件类型配置
#[derive(Clone)]
pub struct FileTypeConfig {
    /// 图片类型
    pub image: FileTypeGroup,
    /// 文档类型
    pub document: FileTypeGroup,
    /// 压缩类型
    pub archive: FileTypeGroup,
    /// 音频类型
    pub audio: FileTypeGroup,
    /// 视频类型
    pub video: FileTypeGroup,
}

/// 文件类型分组
#[derive(Clone)]
pub struct FileTypeGroup {
    /// 文件扩展名
    pub extensions: Vec<String>,
    /// MIME类型
    pub mime_types: Vec<String>,
}

/// 获取文件类型配置
pub fn get_file_type_config() -> Result<FileTypeConfig> {
    Ok(FileTypeConfig {
        image: FileTypeGroup {
            extensions: read_global_array_config!("file_types", "image", "extensions"),
            mime_types: read_global_array_config!("file_types", "image", "mime_types"),
        },
        document: FileTypeGroup {
            extensions: read_global_array_config!("file_types", "document", "extensions"),
            mime_types: read_global_array_config!("file_types", "document", "mime_types"),
        },
        archive: FileTypeGroup {
            extensions: read_global_array_config!("file_types", "archive", "extensions"),
            mime_types: read_global_array_config!("file_types", "archive", "mime_types"),
        },
        audio: FileTypeGroup {
            extensions: read_global_array_config!("file_types", "audio", "extensions"),
            mime_types: read_global_array_config!("file_types", "audio", "mime_types"),
        },
        video: FileTypeGroup {
            extensions: read_global_array_config!("file_types", "video", "extensions"),
            mime_types: read_global_array_config!("file_types", "video", "mime_types"),
        },
    })
}
