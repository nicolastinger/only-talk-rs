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

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use common::config_manager::{remove_config, set_array_config};

    use super::*;

    // GLOBAL_CONFIG 为进程内共享,串行化两个读写同一组 key 的测试避免竞态
    static CONFIG_LOCK: Mutex<()> = Mutex::new(());

    fn install_file_types_config() {
        let groups = [
            ("image", vec!["png", "jpg"], vec!["image/png", "image/jpeg"]),
            ("document", vec!["pdf"], vec!["application/pdf"]),
            ("archive", vec!["zip", "rar"], vec!["application/zip"]),
            ("audio", vec!["mp3"], vec!["audio/mpeg"]),
            ("video", vec!["mp4"], vec!["video/mp4"]),
        ];
        for (group, extensions, mime_types) in groups {
            set_array_config(
                format!("file_types.{}.extensions", group),
                extensions.into_iter().map(|s| s.to_string()).collect(),
            );
            set_array_config(
                format!("file_types.{}.mime_types", group),
                mime_types.into_iter().map(|s| s.to_string()).collect(),
            );
        }
    }

    fn remove_file_types_config() {
        for group in ["image", "document", "archive", "audio", "video"] {
            remove_config(&format!("file_types.{}.extensions", group));
            remove_config(&format!("file_types.{}.mime_types", group));
        }
    }

    #[test]
    fn reads_all_groups_from_config() {
        let _guard = CONFIG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        install_file_types_config();
        let config = get_file_type_config().expect("读取文件类型配置失败");

        assert_eq!(config.image.extensions, vec!["png", "jpg"]);
        assert_eq!(config.image.mime_types, vec!["image/png", "image/jpeg"]);
        assert_eq!(config.document.extensions, vec!["pdf"]);
        assert_eq!(config.document.mime_types, vec!["application/pdf"]);
        assert_eq!(config.archive.extensions, vec!["zip", "rar"]);
        assert_eq!(config.audio.extensions, vec!["mp3"]);
        assert_eq!(config.video.extensions, vec!["mp4"]);
        assert_eq!(config.video.mime_types, vec!["video/mp4"]);

        remove_file_types_config();
    }

    #[test]
    fn missing_config_returns_error() {
        let _guard = CONFIG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        remove_file_types_config();
        assert!(get_file_type_config().is_err());
    }
}
