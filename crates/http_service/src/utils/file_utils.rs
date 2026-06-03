use anyhow::{anyhow, Context};
use image::DynamicImage;
use rbatis::rbdc::rt::tokio;
use std::path::Path;
use tracing::info;
use webp::Encoder;

/// Compress image file to keep file size within specified limit
///
/// # Arguments
/// - `file_path`: Image file path
/// - `target_size_bytes`: Target file size in bytes, default 1MB (1048576 bytes)
///
/// # Returns
/// - `Ok(Vec<u8>)`: Compressed image data (returns original if already smaller than target)
/// - `Err(anyhow::Error)`: Error on compression failure
pub async fn compress_image(
    file_path: &str,
    target_size_bytes: Option<usize>,
) -> Result<Vec<u8>, anyhow::Error> {
    const DEFAULT_TARGET_SIZE: usize = 1024 * 1024; // 1MB
    const MIN_QUALITY: f32 = 30.0; // WebP minimum quality
    const INITIAL_QUALITY: f32 = 80.0; // WebP initial quality
    const ENCODING_METHOD: i32 = 0; // WebP encoding method: 0=fastest, 6=slowest

    let target_size = target_size_bytes.unwrap_or(DEFAULT_TARGET_SIZE);

    // Read file
    let file_data = tokio::fs::read(file_path)
        .await
        .with_context(|| format!("无法读取文件: {}", file_path))?;

    // Check original file size
    if file_data.len() <= target_size {
        info!("file size {} is smaller than target size {}, skipping compression", file_data.len(), target_size);
        return Ok(file_data);
    }

    // Load image
    let img = image::load_from_memory(&file_data)
        .with_context(|| format!("无法加载图片文件: {}", file_path))?;

    info!(
        "开始压缩图片: {}, 原始大小: {} bytes",
        file_path,
        file_data.len()
    );

    // Try reducing dimensions first (faster for large files)
    if file_data.len() > 3 * target_size {
        // 如果原始文件远大于目标，先缩小尺寸
        if let Ok(compressed) = try_compress_by_resize_fast(&img, target_size, INITIAL_QUALITY, ENCODING_METHOD) {
            info!("compression by resizing successful, compressed size: {} bytes", compressed.len());
            return Ok(compressed);
        }
    }

    // Try reducing quality (using binary search)
    if let Ok(compressed) = try_compress_by_quality_binary(&img, target_size, INITIAL_QUALITY, MIN_QUALITY, ENCODING_METHOD) {
        info!("compression successful, compressed size: {} bytes", compressed.len());
        return Ok(compressed);
    }

    // If all methods fail, return image with minimum quality
    let compressed = compress_image_with_quality(&img, MIN_QUALITY, ENCODING_METHOD)?;
    info!("using minimum quality compression, final size: {} bytes", compressed.len());
    Ok(compressed)
}

/// Compress image by reducing WebP quality (using binary search optimization)
fn try_compress_by_quality_binary(
    img: &DynamicImage,
    target_size: usize,
    initial_quality: f32,
    min_quality: f32,
    encoding_method: i32,
) -> Result<Vec<u8>, anyhow::Error> {
    // Try initial quality first
    let initial_compressed = compress_image_with_quality(img, initial_quality, encoding_method)?;
    if initial_compressed.len() <= target_size {
        return Ok(initial_compressed);
    }

    // If initial quality doesn't satisfy, use binary search to find appropriate quality
    let mut low = min_quality;
    let mut high = initial_quality;
    let mut best_result = initial_compressed;

    // Binary search, max 5 iterations (balance speed and precision)
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

/// Compress image by reducing WebP quality (linear method, kept for compatibility)
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

        // Reduce quality, step size 5
        quality = (quality - 5.0).max(min_quality);
    }
}

/// Compress image by reducing dimensions (fast version, using Triangle filter)
fn try_compress_by_resize_fast(
    img: &DynamicImage,
    target_size: usize,
    quality: f32,
    encoding_method: i32,
) -> Result<Vec<u8>, anyhow::Error> {
    let mut img = img.clone();
    let scale = 0.8; // Scale down 20% each iteration, faster

    // Max 4 iterations (avoid excessive looping)
    for _ in 0..4 {
        let new_width = (img.width() as f32 * scale) as u32;
        let new_height = (img.height() as f32 * scale) as u32;

        // Prevent image from becoming too small
        if new_width < 200 || new_height < 200 {
            break;
        }

        // Use Triangle filter (much faster than Lanczos3)
        img = img.resize(new_width, new_height, image::imageops::FilterType::Triangle);

        let compressed = compress_image_with_quality(&img, quality, encoding_method)?;

        if compressed.len() <= target_size {
            return Ok(compressed);
        }
    }

    Err(anyhow!("无法通过缩小尺寸将图片压缩到目标大小"))
}

/// Compress image by reducing dimensions (high quality version, using Lanczos3 filter)
#[allow(dead_code)]
fn try_compress_by_resize(
    img: &DynamicImage,
    target_size: usize,
    quality: f32,
    encoding_method: i32,
) -> Result<Vec<u8>, anyhow::Error> {
    let mut img = img.clone();
    let mut scale = 0.9; // Scale down 10% each iteration

    loop {
        // Scale down image proportionally
        let new_width = (img.width() as f32 * scale) as u32;
        let new_height = (img.height() as f32 * scale) as u32;

        // Prevent image from becoming too small
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

/// Compress image to WebP format with specified quality (fastest encoding speed)
fn compress_image_with_quality(img: &DynamicImage, quality: f32, _encoding_method: i32) -> Result<Vec<u8>, anyhow::Error> {
    // Convert to RGBA format
    let rgba_image = img.to_rgba8();
    let (width, height) = (img.width(), img.height());

    // Use webp crate Encoder
    let encoder = Encoder::from_rgba(&rgba_image, width, height);

    // Encode to WebP
    // Convert quality to 0.0-1.0 range
    let quality_normalized = (quality / 100.0).clamp(0.0, 1.0);
    let webp_data = encoder.encode(quality_normalized);

    Ok(webp_data.to_vec())
}

/// Get image file MIME type
///
/// # Arguments
/// - `file_path`: Image file path
///
/// # Returns
/// - `Option<String>`: MIME type string, None if not a supported image format
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

/// Check if file is an image
///
/// # Arguments
/// - `file_path`: File path
///
/// # Returns
/// - `bool`: true if image file, false otherwise
pub fn is_image_file(file_path: &str) -> bool {
    get_image_mime_type(file_path).is_some()
}
