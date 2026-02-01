use anyhow::{anyhow, Context};
use image::DynamicImage;
use rbatis::rbdc::rt::tokio;
use std::path::Path;
use tracing::info;
use webp::Encoder;

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
    const MIN_QUALITY: f32 = 30.0; // WebP最小质量
    const INITIAL_QUALITY: f32 = 80.0; // WebP初始质量
    const ENCODING_METHOD: i32 = 0; // WebP编码方法: 0=最快, 6=最慢

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

    // 优先尝试缩小尺寸（对于大文件更快）
    if file_data.len() > 3 * target_size {
        // 如果原始文件远大于目标，先缩小尺寸
        if let Ok(compressed) = try_compress_by_resize_fast(&img, target_size, INITIAL_QUALITY, ENCODING_METHOD) {
            info!("通过缩小尺寸压缩成功，压缩后大小: {} bytes", compressed.len());
            return Ok(compressed);
        }
    }

    // 尝试通过降低质量来压缩（使用二分法）
    if let Ok(compressed) = try_compress_by_quality_binary(&img, target_size, INITIAL_QUALITY, MIN_QUALITY, ENCODING_METHOD) {
        info!("压缩成功，压缩后大小: {} bytes", compressed.len());
        return Ok(compressed);
    }

    // 如果所有方法都失败，返回质量最低的图片
    let compressed = compress_image_with_quality(&img, MIN_QUALITY, ENCODING_METHOD)?;
    info!("使用最低质量压缩，最终大小: {} bytes", compressed.len());
    Ok(compressed)
}

/// 通过降低 WebP 质量来压缩图片（使用二分法优化）
fn try_compress_by_quality_binary(
    img: &DynamicImage,
    target_size: usize,
    initial_quality: f32,
    min_quality: f32,
    encoding_method: i32,
) -> Result<Vec<u8>, anyhow::Error> {
    // 先尝试初始质量
    let initial_compressed = compress_image_with_quality(img, initial_quality, encoding_method)?;
    if initial_compressed.len() <= target_size {
        return Ok(initial_compressed);
    }

    // 如果初始质量不满足，使用二分法查找合适质量
    let mut low = min_quality;
    let mut high = initial_quality;
    let mut best_result = initial_compressed;

    // 二分查找，最多尝试5次（平衡速度和精度）
    for _ in 0..5 {
        let mid = (low + high) / 2.0;
        let compressed = compress_image_with_quality(img, mid, encoding_method)?;

        if compressed.len() <= target_size {
            best_result = compressed;
            low = mid + 5.0; // 尝试更高的质量
        } else {
            high = mid - 5.0;
        }
    }

    Ok(best_result)
}

/// 通过降低 WebP 质量来压缩图片（线性方法，保留用于兼容）
#[allow(dead_code)]
fn try_compress_by_quality(
    img: &DynamicImage,
    target_size: usize,
    initial_quality: f32,
    min_quality: f32,
    encoding_method: i32,
) -> Result<Vec<u8>, anyhow::Error> {
    let mut quality = initial_quality;

    loop {
        let compressed = compress_image_with_quality(img, quality, encoding_method)?;

        if compressed.len() <= target_size || quality <= min_quality {
            return Ok(compressed);
        }

        // 降低质量，步长为 5
        quality = (quality - 5.0).max(min_quality);
    }
}

/// 通过缩小尺寸来压缩图片（快速版本，使用Triangle滤镜）
fn try_compress_by_resize_fast(
    img: &DynamicImage,
    target_size: usize,
    quality: f32,
    encoding_method: i32,
) -> Result<Vec<u8>, anyhow::Error> {
    let mut img = img.clone();
    let scale = 0.8; // 每次缩小 20%，更快

    // 最多尝试4次（避免过多循环）
    for _ in 0..4 {
        let new_width = (img.width() as f32 * scale) as u32;
        let new_height = (img.height() as f32 * scale) as u32;

        // 防止图片过小
        if new_width < 200 || new_height < 200 {
            break;
        }

        // 使用Triangle滤镜（比Lanczos3快很多）
        img = img.resize(new_width, new_height, image::imageops::FilterType::Triangle);

        let compressed = compress_image_with_quality(&img, quality, encoding_method)?;

        if compressed.len() <= target_size {
            return Ok(compressed);
        }
    }

    Err(anyhow!("无法通过缩小尺寸将图片压缩到目标大小"))
}

/// 通过缩小尺寸来压缩图片（高质量版本，使用Lanczos3滤镜）
#[allow(dead_code)]
fn try_compress_by_resize(
    img: &DynamicImage,
    target_size: usize,
    quality: f32,
    encoding_method: i32,
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

        let compressed = compress_image_with_quality(&img, quality, encoding_method)?;

        if compressed.len() <= target_size {
            return Ok(compressed);
        }

        scale -= 0.1;
    }

    Err(anyhow!("无法通过缩小尺寸将图片压缩到目标大小"))
}

/// 使用指定质量压缩图片为 WebP 格式（最快编码速度）
fn compress_image_with_quality(img: &DynamicImage, quality: f32, _encoding_method: i32) -> Result<Vec<u8>, anyhow::Error> {
    // 转换为RGBA格式
    let rgba_image = img.to_rgba8();
    let (width, height) = (img.width(), img.height());

    // 使用webp crate的Encoder
    let encoder = Encoder::from_rgba(&rgba_image, width, height);

    // 编码为WebP
    // quality转换为0.0-1.0范围
    let quality_normalized = (quality / 100.0).clamp(0.0, 1.0);
    let webp_data = encoder.encode(quality_normalized);

    Ok(webp_data.to_vec())
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
