// logger.rs - Structured logging module

use log::{debug, error, info, warn, LevelFilter};
use serde::Serialize;
use std::sync::Once;

static INIT: Once = Once::new();

#[derive(Debug, Clone)]
pub struct Logger {
    _enabled: bool,
    _max_level: LevelFilter,
}

impl Logger {
    pub fn init(level: &str, _enabled: bool) {
        INIT.call_once(|| {
            let level_filter = match level.to_uppercase().as_str() {
                "DEBUG" => LevelFilter::Debug,
                "TRACE" => LevelFilter::Trace,
                "WARN" => LevelFilter::Warn,
                "ERROR" => LevelFilter::Error,
                _ => LevelFilter::Info,
            };

            env_logger::Builder::new()
                .filter_level(level_filter)
                .format_timestamp(None)
                .init();
        });
    }

    pub fn new(level: &str, enabled: bool) -> Self {
        let max_level = match level.to_uppercase().as_str() {
            "DEBUG" => LevelFilter::Debug,
            "TRACE" => LevelFilter::Trace,
            "WARN" => LevelFilter::Warn,
            "ERROR" => LevelFilter::Error,
            _ => LevelFilter::Info,
        };

        Logger { _enabled: enabled, _max_level: max_level }
    }

    pub fn format_bytes(&self, bytes: u64) -> String {
        if bytes == 0 {
            return "0 Bytes".to_string();
        }

        let sizes = ["Bytes", "KB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];
        let i = ((bytes as f64).ln() / 1024.0_f64.ln()).floor() as usize;
        let size = bytes as f64 / 1024.0_f64.powi(i as i32);

        format!("{:.2} {}", size, sizes[i.min(sizes.len() - 1)])
    }

    fn truncate_url(&self, url: &str, max_length: usize) -> String {
        if url.len() > max_length {
            format!("{}...", &url[..max_length.saturating_sub(3)])
        } else {
            url.to_string()
        }
    }

    fn truncate_string(&self, s: &str, max_length: usize) -> String {
        if s.len() > max_length {
            format!("{}...", &s[..max_length.saturating_sub(3)])
        } else {
            s.to_string()
        }
    }

    pub fn log_compression_process(
        &self,
        _url: &str,
        original_size: u64,
        compressed_size: Option<u64>,
        bytes_saved: Option<u64>,
        quality: u8,
        format: &str,
        error: Option<&str>,
    ) {
        if let Some(err) = error {
            warn!("compress: FAILED - {}", err);
        } else if let (Some(comp_size), Some(saved)) = (compressed_size, bytes_saved) {
            let percent = if original_size > 0 {
                format!("{:.1}%", ((original_size - comp_size) as f64 / original_size as f64) * 100.0)
            } else {
                "0.0%".to_string()
            };

            info!("compress: {} - S: {}/{} Q: {}", format, saved, percent, quality);
        }
    }

    pub fn log_request(
        &self,
        url: &str,
        user_agent: Option<&str>,
        referer: Option<&str>,
        ip: Option<&str>,
        jpeg: Option<&str>,
        bw: Option<&str>,
        quality: u8,
        content_type: Option<&str>,
    ) {
        debug!(
            "Request received: {}",
            serde_json::json!({
                "url": self.truncate_url(url, 20),
                "client": {
                    "ip": ip.unwrap_or("Unknown"),
                    "userAgent": self.truncate_string(user_agent.unwrap_or(""), 100),
                    "referer": referer.unwrap_or("Direct"),
                },
                "compressionOptions": {
                    "forceJpeg": jpeg.is_some(),
                    "grayscale": bw.is_some(),
                    "quality": quality,
                },
                "contentType": content_type.unwrap_or("Unknown"),
            })
        );
    }

    pub fn log_bypass(&self, url: &str, size: u64, reason: &str) {
        info!(
            "Bypassing: {}",
            serde_json::json!({
                "url": self.truncate_url(url, 20),
                "size": self.format_bytes(size),
                "reason": reason,
            })
        );
    }

    pub fn log_upstream_fetch(&self, url: &str, status_code: u16, success: bool) {
        let status_icon = if success { "✓" } else { "✗" };
        let truncated_url = self.truncate_url(url, 60);
        
        if success {
            info!("fetch [{}] {} - {}", status_icon, status_code, truncated_url);
        } else {
            warn!("fetch [{}] {} - {}", status_icon, status_code, truncated_url);
        }
    }

    pub fn error<T: Serialize>(&self, message: &str, metadata: &T) {
        error!("{}: {}", message, serde_json::to_string(metadata).unwrap_or_default());
    }

    #[allow(dead_code)]
    pub fn warn<T: Serialize>(&self, message: &str, metadata: &T) {
        warn!("{}: {}", message, serde_json::to_string(metadata).unwrap_or_default());
    }

    pub fn info<T: Serialize>(&self, message: &str, metadata: &T) {
        info!("{}: {}", message, serde_json::to_string(metadata).unwrap_or_default());
    }

    pub fn debug<T: Serialize>(&self, message: &str, metadata: &T) {
        debug!("{}: {}", message, serde_json::to_string(metadata).unwrap_or_default());
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::new("INFO", true)
    }
}
