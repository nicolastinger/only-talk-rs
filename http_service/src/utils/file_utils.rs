use anyhow::{anyhow, Context};
use image::codecs::jpeg::JpegEncoder;
use image::DynamicImage;
use rbatis::rbdc::rt::tokio;
use std::io::Cursor;
use std::path::Path;
use tracing::info;

/// 压缩图片文件，将文件大小控制在指定限制以内
///
/// # 参数
/// - `file_path`: 图片文件路径
/// - `target_size_bytes`: 目标文件大小（字节），默认为 1MB (1048576 字节)
///
/// # 返回
/// - `Ok(Vec<u8>)`: 压缩后的图片数据（如果文件已小于目标大小则返回原数据）
/// - `Err(anyhow::Error)`: 压缩失败时返回错误
pub async fn compress_image(
    file_path: &str,
    target_size_bytes: Option<usize>,
) -> Result<Vec<u8>, anyhow::Error> {
    const DEFAULT_TARGET_SIZE: usize = 1024 * 1024; // 1MB
    const MIN_QUALITY: u8 = 10; // 最小质量
    const INITIAL_QUALITY: u8 = 85; // 初始质量

    let target_size = target_size_bytes.unwrap_or(DEFAULT_TARGET_SIZE);

    // 读取文件
    let file_data = tokio::fs::read(file_path)
        .await
        .with_context(|| format!("无法读取文件: {}", file_path))?;

    // 检查原始文件大小
    if file_data.len() <= target_size {
        info!("文件大小 {} 小于目标大小 {}，无需压缩", file_data.len(), target_size);
        return Ok(file_data);
    }

    // 加载图片
    let img = image::load_from_memory(&file_data)
        .with_context(|| format!("无法加载图片文件: {}", file_path))?;

    info!(
        "开始压缩图片: {}, 原始大小: {} bytes",
        file_path,
        file_data.len()
    );

    // 尝试通过降低质量来压缩
    if let Ok(compressed) = try_compress_by_quality(&img, target_size, INITIAL_QUALITY, MIN_QUALITY) {
        info!("压缩成功，压缩后大小: {} bytes", compressed.len());
        return Ok(compressed);
    }

    // 如果降低质量仍然无法达到目标，尝试缩小尺寸
    if let Ok(compressed) = try_compress_by_resize(&img, target_size, INITIAL_QUALITY) {
        info!("通过缩小尺寸压缩成功，压缩后大小: {} bytes", compressed.len());
        return Ok(compressed);
    }

    // 如果所有方法都失败，返回质量最低的图片
    let compressed = compress_image_with_quality(&img, MIN_QUALITY)?;
    info!("使用最低质量压缩，最终大小: {} bytes", compressed.len());
    Ok(compressed)
}

/// 通过降低 JPEG 质量来压缩图片
fn try_compress_by_quality(
    img: &DynamicImage,
    target_size: usize,
    initial_quality: u8,
    min_quality: u8,
) -> Result<Vec<u8>, anyhow::Error> {
    let mut quality = initial_quality;

    loop {
        let compressed = compress_image_with_quality(img, quality)?;

        if compressed.len() <= target_size || quality <= min_quality {
            return Ok(compressed);
        }

        // 降低质量，步长为 5
        quality = quality.saturating_sub(5);
    }
}

/// 通过缩小尺寸来压缩图片
fn try_compress_by_resize(
    img: &DynamicImage,
    target_size: usize,
    quality: u8,
) -> Result<Vec<u8>, anyhow::Error> {
    let mut img = img.clone();
    let mut scale = 0.9; // 每次缩小 10%

    loop {
        // 按比例缩小图片
        let new_width = (img.width() as f32 * scale) as u32;
        let new_height = (img.height() as f32 * scale) as u32;

        // 防止图片过小
        if new_width < 100 || new_height < 100 {
            break;
        }

        img = img.resize(new_width, new_height, image::imageops::FilterType::Lanczos3);

        let compressed = compress_image_with_quality(&img, quality)?;

        if compressed.len() <= target_size {
            return Ok(compressed);
        }

        scale -= 0.1;
    }

    Err(anyhow!("无法通过缩小尺寸将图片压缩到目标大小"))
}

/// 使用指定质量压缩图片为 JPEG 格式
fn compress_image_with_quality(img: &DynamicImage, quality: u8) -> Result<Vec<u8>, anyhow::Error> {
    let mut buffer = Cursor::new(Vec::new());

    let encoder = JpegEncoder::new_with_quality(&mut buffer, quality);
    img.write_with_encoder(encoder)
        .map_err(|e| anyhow!("图片编码失败: {}", e))?;

    Ok(buffer.into_inner())
}

/// 获取图片文件的 MIME 类型
///
/// # 参数
/// - `file_path`: 图片文件路径
///
/// # 返回
/// - `Option<String>`: MIME 类型字符串，如果不是支持的图片格式则返回 None
pub fn get_image_mime_type(file_path: &str) -> Option<String> {
    let path = Path::new(file_path);
    let extension = path.extension()?.to_str()?;

    match extension.to_lowercase().as_str() {
        "jpg" | "jpeg" => Some("image/jpeg".to_string()),
        "png" => Some("image/png".to_string()),
        "gif" => Some("image/gif".to_string()),
        "webp" => Some("image/webp".to_string()),
        "bmp" => Some("image/bmp".to_string()),
        _ => None,
    }
}

/// 检查文件是否为图片
///
/// # 参数
/// - `file_path`: 文件路径
///
/// # 返回
/// - `bool`: 如果是图片文件返回 true，否则返回 false
pub fn is_image_file(file_path: &str) -> bool {
    get_image_mime_type(file_path).is_some()
}
