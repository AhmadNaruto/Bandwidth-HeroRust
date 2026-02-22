// compress.rs - Image compression module

use image::{DynamicImage, GenericImageView, ImageFormat, ImageReader};
use std::io::Cursor;

#[cfg(feature = "avif")]
use ravif::{Encoder, AlphaColorMode, BitDepth};
#[cfg(feature = "avif")]
use rgb::RGBA8;

use crate::logger::Logger;

/// Configuration constants for compression
pub struct Config {
    pub max_width: u32,
    pub max_jpeg_height: u32,
    pub max_avif_height: u32,
    pub grayscale_quality_range: (u8, u8),
}

impl Default for Config {
    fn default() -> Self {
        Config {
            max_width: 400,
            max_jpeg_height: 32767,
            max_avif_height: 16383,
            grayscale_quality_range: (10, 40),
        }
    }
}

/// Result of compression operation
#[derive(Debug)]
pub struct CompressionResult {
    pub data: Vec<u8>,
    pub format: String,
    pub bytes_saved: i64,
}

/// Error types for compression
#[derive(Debug, thiserror::Error)]
pub enum CompressionError {
    #[error("Image processing error: {0}")]
    ImageError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Calculate new dimensions maintaining aspect ratio
fn calculate_dimensions(
    width: u32,
    height: u32,
    max_width: u32,
) -> (u32, u32) {
    if width <= max_width {
        return (width, height);
    }

    let ratio = max_width as f32 / width as f32;
    (
        (width as f32 * ratio).round() as u32,
        (height as f32 * ratio).round() as u32,
    )
}

/// Select the best output format based on image properties
fn select_format(
    use_avif: bool,
    calculated_height: u32,
    config: &Config,
) -> ImageFormat {
    if calculated_height > config.max_jpeg_height {
        return ImageFormat::Jpeg;
    }

    if use_avif && calculated_height > config.max_avif_height {
        return ImageFormat::Jpeg;
    }

    if use_avif {
        ImageFormat::Avif
    } else {
        ImageFormat::Jpeg
    }
}

/// Compress image to JPEG format
fn compress_jpeg(
    img: &DynamicImage,
    quality: u8,
    grayscale: bool,
) -> Result<Vec<u8>, CompressionError> {
    let processed_img = if grayscale {
        img.grayscale()
    } else {
        img.clone()
    };

    let mut buffer = Vec::new();
    let mut cursor = Cursor::new(&mut buffer);
    
    // Create JPEG encoder with quality
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, quality);
    
    // Encode the image
    encoder.encode(
        processed_img.as_bytes(),
        processed_img.width(),
        processed_img.height(),
        processed_img.color().into(),
    )
    .map_err(|e| CompressionError::ImageError(e.to_string()))?;

    Ok(buffer)
}

/// Compress image to AVIF format
#[cfg(feature = "avif")]
fn compress_avif(
    img: &DynamicImage,
    quality: u8,
    grayscale: bool,
) -> Result<Vec<u8>, CompressionError> {
    let processed_img = if grayscale {
        img.grayscale()
    } else {
        img.clone()
    };

    let rgba = processed_img.to_rgba8();
    let (width, height) = rgba.dimensions();

    // Convert to RGBA8 format expected by ravif
    let rgba_data: Vec<RGBA8> = rgba
        .chunks_exact(4)
        .map(|chunk| RGBA8::new(chunk[0], chunk[1], chunk[2], chunk[3]))
        .collect();

    let result = Encoder::new()
        .with_quality(quality as f32)
        .with_speed(4)
        .with_bit_depth(BitDepth::Eight)
        .with_alpha_color_mode(AlphaColorMode::UnassociatedDirty)
        .encode_rgba(imgref::Img::new(rgba_data.as_slice(), width as usize, height as usize))
        .map_err(|e| CompressionError::ImageError(e.to_string()))?;

    Ok(result.avif_file)
}

/// Compress image to AVIF format (fallback without ravif)
#[cfg(not(feature = "avif"))]
fn compress_avif(
    img: &DynamicImage,
    quality: u8,
    grayscale: bool,
) -> Result<Vec<u8>, CompressionError> {
    // Fallback to JPEG if AVIF not available
    compress_jpeg(img, quality, grayscale)
}

/// Main compression function
pub async fn compress(
    image_data: &[u8],
    use_avif: bool,
    grayscale: bool,
    quality: u8,
    original_size: u64,
    logger: &Logger,
) -> Result<CompressionResult, CompressionError> {
    let config = Config::default();

    logger.debug(
        "Compression started",
        &serde_json::json!({
            "originalSize": original_size,
            "quality": quality,
            "useAvif": use_avif,
            "grayscale": grayscale,
        }),
    );

    // Load image
    let img = ImageReader::new(Cursor::new(image_data))
        .with_guessed_format()
        .map_err(|e| CompressionError::ImageError(e.to_string()))?
        .decode()
        .map_err(|e| CompressionError::ImageError(e.to_string()))?;

    // Calculate dimensions
    let (orig_width, orig_height) = img.dimensions();
    let (new_width, new_height) = calculate_dimensions(orig_width, orig_height, config.max_width);

    logger.debug(
        "Image dimensions",
        &serde_json::json!({
            "original": {"width": orig_width, "height": orig_height},
            "resized": {"width": new_width, "height": new_height},
        }),
    );

    // Resize image
    let resized = img.resize_exact(
        new_width,
        new_height,
        image::imageops::FilterType::Lanczos3,
    );

    // Select output format
    let output_format = select_format(use_avif, new_height, &config);

    // Calculate effective quality for grayscale
    let effective_quality = if grayscale {
        quality.clamp(config.grayscale_quality_range.0, config.grayscale_quality_range.1)
    } else {
        quality
    };

    // Compress based on format
    let compressed_data = match output_format {
        ImageFormat::Avif => compress_avif(&resized, effective_quality, grayscale)?,
        ImageFormat::Jpeg => compress_jpeg(&resized, effective_quality, grayscale)?,
        _ => compress_jpeg(&resized, effective_quality, grayscale)?,
    };

    let compressed_size = compressed_data.len() as u64;
    let bytes_saved = original_size as i64 - compressed_size as i64;

    // Check if compression was beneficial
    if compressed_size > original_size {
        logger.log_compression_process(
            "unknown",
            original_size,
            Some(compressed_size),
            Some(0),
            quality,
            &format!("{:?}", output_format),
            Some("bypassed-larger"),
        );

        // Return original data
        return Ok(CompressionResult {
            data: image_data.to_vec(),
            format: "original".to_string(),
            bytes_saved: 0,
        });
    }

    let format_str = match output_format {
        ImageFormat::Avif => "avif",
        ImageFormat::Jpeg => "jpeg",
        _ => "jpeg",
    };

    logger.log_compression_process(
        "unknown",
        original_size,
        Some(compressed_size),
        Some(bytes_saved as u64),
        quality,
        format_str,
        None,
    );

    Ok(CompressionResult {
        data: compressed_data,
        format: format_str.to_string(),
        bytes_saved,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_dimensions() {
        assert_eq!(calculate_dimensions(800, 600, 400), (400, 300));
        assert_eq!(calculate_dimensions(400, 300, 400), (400, 300));
        assert_eq!(calculate_dimensions(200, 150, 400), (200, 150));
    }

    #[test]
    fn test_select_format_height_limit() {
        let config = Config::default();
        assert_eq!(
            select_format(true, 40000, &config),
            ImageFormat::Jpeg
        );
        assert_eq!(
            select_format(true, 1000, &config),
            ImageFormat::Avif
        );
        assert_eq!(
            select_format(false, 1000, &config),
            ImageFormat::Jpeg
        );
    }
}
