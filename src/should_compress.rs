// should_compress.rs - Determines if an image should be compressed

/// Configuration constants for compression decisions
pub struct Config {
    pub min_compress_length: u64,
    pub min_transparent_compress_length: u64,
    pub max_original_size: u64,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            min_compress_length: 2048,
            min_transparent_compress_length: 102400,
            max_original_size: 5 * 1024 * 1024,
        }
    }
}

/// Determines if an image should be compressed based on type, size, and transparency
pub fn should_compress(
    image_type: &str,
    size: u64,
    is_transparent: bool,
    config: &Config,
) -> bool {
    if image_type.is_empty() {
        return false;
    }

    // Check size constraints
    if size > config.max_original_size || size < config.min_compress_length {
        return false;
    }

    // Check if it's a supported image type
    if !is_supported_image_type(image_type) {
        return false;
    }

    // Handle transparent images
    if is_transparent {
        return size >= config.min_compress_length;
    }

    // For non-transparent PNG/GIF, ensure they're large enough
    if image_type.ends_with("png") || image_type.ends_with("gif") {
        return size >= config.min_transparent_compress_length;
    }

    true
}

/// Check if the MIME type is a supported image format
fn is_supported_image_type(image_type: &str) -> bool {
    let supported = [
        "image/jpeg",
        "image/png",
        "image/gif",
        "image/webp",
        "image/bmp",
        "image/tiff",
    ];
    supported.iter().any(|&t| image_type.eq_ignore_ascii_case(t))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_compress_valid_image() {
        let config = Config::default();
        assert!(should_compress("image/jpeg", 5000, false, &config));
        assert!(should_compress("image/png", 150000, false, &config));
    }

    #[test]
    fn test_should_compress_too_small() {
        let config = Config::default();
        assert!(!should_compress("image/jpeg", 1000, false, &config));
    }

    #[test]
    fn test_should_compress_too_large() {
        let config = Config::default();
        assert!(!should_compress("image/jpeg", 6 * 1024 * 1024, false, &config));
    }

    #[test]
    fn test_should_compress_unsupported_type() {
        let config = Config::default();
        assert!(!should_compress("image/svg+xml", 5000, false, &config));
    }

    #[test]
    fn test_should_compress_transparent() {
        let config = Config::default();
        assert!(should_compress("image/png", 50000, true, &config));
        assert!(!should_compress("image/png", 5000, true, &config));
    }
}
