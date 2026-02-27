// logger.rs - Structured logging module with modern display

use log::{debug, error, info, warn, LevelFilter};
use serde::Serialize;
use std::sync::Once;

static INIT: Once = Once::new();

/// ANSI color codes for modern terminal output
mod colors {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const DIM: &str = "\x1b[2m";
    pub const BLUE: &str = "\x1b[34m";
    pub const GREEN: &str = "\x1b[32m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const RED: &str = "\x1b[31m";
    pub const CYAN: &str = "\x1b[36m";
    pub const MAGENTA: &str = "\x1b[35m";
    pub const WHITE: &str = "\x1b[37m";
    pub const BG_BLUE: &str = "\x1b[44m";
    pub const BG_GREEN: &str = "\x1b[42m";
    pub const BG_YELLOW: &str = "\x1b[43m";
    pub const BG_RED: &str = "\x1b[41m";
    pub const BG_MAGENTA: &str = "\x1b[45m";
}

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
                .format_module_path(false)
                .format_target(false)
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
            return "0 B".to_string();
        }

        let sizes = ["B", "KB", "MB", "GB", "TB"];
        let i = ((bytes as f64).ln() / 1024.0_f64.ln()).floor() as usize;
        let size = bytes as f64 / 1024.0_f64.powi(i as i32);

        format!("{:.1} {}", size, sizes[i.min(sizes.len() - 1)])
    }

    fn truncate_url(&self, url: &str, max_length: usize) -> String {
        if url.len() > max_length {
            format!("{}...", &url[..max_length.saturating_sub(3)])
        } else {
            url.to_string()
        }
    }

    /// Extract domain and filename from URL for display
    fn format_url_for_display(&self, url: &str) -> String {
        // Parse URL to get domain and path
        if let Ok(parsed) = url::Url::parse(url) {
            let domain = parsed.host_str().unwrap_or("unknown");
            let path = parsed.path();
            
            // Extract filename from path
            let filename = path
                .split('/')
                .filter(|s| !s.is_empty())
                .last()
                .unwrap_or("");
            
            if filename.is_empty() {
                // No filename, use domain only
                domain.to_string()
            } else {
                format!("{} > {}", domain, filename)
            }
        } else {
            // Fallback to truncated URL if parsing fails
            self.truncate_url(url, 50)
        }
    }

    pub fn log_compression_process(
        &self,
        _url: &str,
        original_size: u64,
        compressed_size: Option<u64>,
        _bytes_saved: Option<u64>,
        quality: u8,
        format: &str,
        error: Option<&str>,
    ) {
        use colors::*;

        if let Some(err) = error {
            let msg = String::new() 
                + BG_RED + WHITE + BOLD + " ✗ ERROR " + RESET + " " + RED + err + RESET;
            warn!("{}", msg);
        } else if let (Some(comp_size), Some(_saved)) = (compressed_size, _bytes_saved) {
            let percent = if original_size > 0 {
                ((original_size - comp_size) as f64 / original_size as f64) * 100.0
            } else {
                0.0
            };

            let format_badge = match format {
                "avif" => String::new() + BG_BLUE + WHITE + BOLD + " AVIF " + RESET,
                "jpeg" => String::new() + BG_YELLOW + WHITE + BOLD + " JPEG " + RESET,
                _ => String::new() + BG_BLUE + WHITE + BOLD + " " + &format.to_uppercase() + " " + RESET,
            };

            let msg = format_badge 
                + " " + DIM + "compress" + RESET 
                + " " + WHITE + &self.format_bytes(original_size) + RESET
                + " " + DIM + "→" + RESET 
                + " " + GREEN + &self.format_bytes(comp_size) + RESET
                + " " + CYAN + &format!("(-{:.1}%)", percent) + RESET
                + " " + DIM + &format!("Q:{}", quality) + RESET;
            info!("{}", msg);
        }
    }

    pub fn log_request(
        &self,
        url: &str,
        _user_agent: Option<&str>,
        _referer: Option<&str>,
        ip: Option<&str>,
        jpeg: Option<&str>,
        bw: Option<&str>,
        quality: u8,
        content_type: Option<&str>,
    ) {
        use colors::*;

        let truncated_url = self.truncate_url(url, 40);
        let jpeg_str = if jpeg.is_some() { "yes" } else { "no" };
        let bw_str = if bw.is_some() { "yes" } else { "no" };
        let jpeg_color = if jpeg.is_some() { GREEN } else { DIM };
        let bw_color = if bw.is_some() { GREEN } else { DIM };

        let msg = String::new()
            + DIM + "━━━━━" + RESET
            + " " + BOLD + CYAN + "REQUEST" + RESET + " "
            + DIM + "━━━━━" + RESET
            + " " + DIM + "URL:" + RESET + " " + BLUE + &truncated_url + RESET
            + " " + DIM + "IP:" + RESET + " " + WHITE + ip.unwrap_or("Unknown") + RESET
            + " " + DIM + "TYPE:" + RESET + " " + WHITE + content_type.unwrap_or("Unknown") + RESET
            + " " + DIM + "JPEG:" + RESET + " " + jpeg_color + jpeg_str + RESET
            + " " + DIM + "BW:" + RESET + " " + bw_color + bw_str + RESET
            + " " + DIM + "Q:" + RESET + " " + MAGENTA + &quality.to_string() + RESET
            + " " + DIM + "━━━━━" + RESET;
        debug!("{}", msg);
    }

    pub fn log_bypass(&self, url: &str, size: u64, reason: &str) {
        use colors::*;

        let reason_badge = match reason {
            "already_small" => String::new() + BG_BLUE + WHITE + BOLD + " SMALL " + RESET,
            "criteria_not_met" => String::new() + BG_YELLOW + WHITE + BOLD + " SKIP " + RESET,
            "non-image" => String::new() + BG_MAGENTA + WHITE + BOLD + " NON-IMG " + RESET,
            _ => String::new() + BG_BLUE + WHITE + BOLD + " " + &reason.to_uppercase() + " " + RESET,
        };

        let msg = reason_badge 
            + " " + DIM + "bypass" + RESET
            + " " + WHITE + &self.format_bytes(size) + RESET
            + " " + DIM + "→" + RESET
            + " " + BLUE + &self.truncate_url(url, 50) + RESET;
        info!("{}", msg);
    }

    pub fn log_upstream_fetch(&self, url: &str, status_code: u16, success: bool) {
        use colors::*;

        let display_url = self.format_url_for_display(url);
        
        let status_color = if status_code >= 200 && status_code < 300 {
            GREEN
        } else if status_code >= 300 && status_code < 400 {
            YELLOW
        } else {
            RED
        };

        let icon = if success { "✓" } else { "✗" };
        let badge = if success {
            String::new() + BG_GREEN + WHITE + BOLD + " " + icon + " " + &status_code.to_string() + " " + RESET
        } else {
            String::new() + BG_RED + WHITE + BOLD + " " + icon + " " + &status_code.to_string() + " " + RESET
        };

        let msg = String::new() + "fetch " + &badge + " " + status_color + &display_url + RESET;
        if success {
            info!("{}", msg);
        } else {
            warn!("{}", msg);
        }
    }

    pub fn error<T: Serialize>(&self, message: &str, metadata: &T) {
        use colors::*;
        let meta = serde_json::to_string(metadata).unwrap_or_default();
        let msg = String::new() 
            + BG_RED + WHITE + BOLD + " ✗ ERROR " + RESET 
            + " " + RED + &format!("{} | {}", message, meta) + RESET;
        error!("{}", msg);
    }

    #[allow(dead_code)]
    pub fn warn<T: Serialize>(&self, message: &str, metadata: &T) {
        use colors::*;
        let meta = serde_json::to_string(metadata).unwrap_or_default();
        let msg = String::new() 
            + BG_YELLOW + WHITE + BOLD + " ⚠ WARN " + RESET 
            + " " + YELLOW + &format!("{} | {}", message, meta) + RESET;
        warn!("{}", msg);
    }

    #[allow(dead_code)]
    pub fn info<T: Serialize>(&self, message: &str, metadata: &T) {
        use colors::*;
        let meta = serde_json::to_string(metadata).unwrap_or_default();
        let msg = String::new() 
            + BG_BLUE + WHITE + BOLD + " ℹ INFO " + RESET 
            + " " + CYAN + &format!("{} | {}", message, meta) + RESET;
        info!("{}", msg);
    }

    pub fn debug<T: Serialize>(&self, message: &str, metadata: &T) {
        use colors::*;
        let meta = serde_json::to_string(metadata).unwrap_or_default();
        let msg = String::new() 
            + BG_MAGENTA + WHITE + BOLD + " ⋯ DEBUG " + RESET 
            + " " + MAGENTA + &format!("{} | {}", message, meta) + RESET;
        debug!("{}", msg);
    }

    /// Log server startup with style
    pub fn log_startup(&self, version: &str, address: &str) {
        use colors::*;
        
        let box_style = String::new() + BOLD + BG_BLUE + WHITE;
        let r = RESET;
        
        eprintln!();
        eprintln!("{box_style} ════════════════════════════════════════════════════ {r}{box_style} ════════════════════════════════════════════════════ {r}");
        eprintln!("{box_style} ║ {r}                                              {box_style} ║ {r}");
        eprintln!("{box_style} ║  {BOLD}{WHITE} 🚀 BANDWIDTH HERO PROXY {r} {box_style}                       {r}{box_style} ║ {r}");
        eprintln!("{box_style} ║  {WHITE}Version: {CYAN}{version}{r} {box_style}                                 {r}{box_style} ║ {r}");
        eprintln!("{box_style} ║  {WHITE}Address: {GREEN}{address}{r} {box_style}                              {r}{box_style} ║ {r}");
        eprintln!("{box_style} ║ {r}                                              {box_style} ║ {r}");
        eprintln!("{box_style} ════════════════════════════════════════════════════ {r}{box_style} ════════════════════════════════════════════════════ {r}");
        eprintln!();
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::new("INFO", true)
    }
}
