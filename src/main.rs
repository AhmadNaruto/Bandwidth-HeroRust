// main.rs - Bandwidth Hero Proxy Server

mod compress;
mod logger;
mod pick;
mod should_compress;

use axum::{
    extract::{Query, State},
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::Response,
    routing::get,
    Json, Router,
};
use md5::{Digest, Md5};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, time::Duration};
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use url::Url;

use crate::compress::compress;
use crate::logger::Logger;
use crate::pick::pick;
use crate::should_compress::{should_compress, Config as CompressConfig};

/// Application state shared across requests
#[derive(Clone)]
struct AppState {
    http_client: Client,
    logger: Logger,
    config: ServerConfig,
}

/// Server configuration
#[derive(Clone, Debug)]
struct ServerConfig {
    port: u16,
    bypass_threshold: u64,
    fetch_headers_to_pick: Vec<&'static str>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            port: std::env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(3000),
            bypass_threshold: 10240,
            fetch_headers_to_pick: vec![
                "cookie",
                "dnt",
                "referer",
                "user-agent",
                "accept",
                "accept-language",
            ],
        }
    }
}

/// Query parameters for the compression endpoint
#[derive(Debug, Deserialize)]
struct CompressionQuery {
    url: Option<String>,
    jpeg: Option<String>,
    bw: Option<String>,
    l: Option<String>,
}

/// Error response
#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
}

/// Cache headers for responses
fn get_cache_headers(custom: Option<HeaderMap>) -> HeaderMap {
    let mut headers = HeaderMap::new();

    headers.insert("content-encoding", HeaderValue::from_static("identity"));
    headers.insert(
        "cache-control",
        HeaderValue::from_static("private, no-store, no-cache, must-revalidate, max-age=0"),
    );
    headers.insert("pragma", HeaderValue::from_static("no-cache"));
    headers.insert("expires", HeaderValue::from_static("0"));
    headers.insert(
        "vary",
        HeaderValue::from_static("url, jpeg, grayscale, quality"),
    );

    if let Some(custom_headers) = custom {
        for (key, value) in custom_headers {
            if let Some(k) = key {
                headers.insert(k, value);
            }
        }
    }

    headers
}

/// Create an error response
fn create_error_response(
    status_code: StatusCode,
    message: &str,
    url: Option<String>,
) -> (StatusCode, Json<ErrorResponse>) {
    (
        status_code,
        Json(ErrorResponse {
            error: message.to_string(),
            url,
        }),
    )
}

/// Create an image response
fn create_image_response(
    buffer: Vec<u8>,
    content_type: &str,
    additional_headers: Option<HeaderMap>,
) -> Response {
    let mut headers = get_cache_headers(additional_headers);

    headers.insert(
        "content-type",
        HeaderValue::from_str(content_type).unwrap_or_else(|_| HeaderValue::from_static("image/jpeg")),
    );

    headers.insert(
        "content-length",
        HeaderValue::from(buffer.len()),
    );

    let mut response = Response::new(buffer.into());
    *response.headers_mut() = headers;
    response
}

/// Parse query parameters
fn parse_query_params(params: &CompressionQuery) -> Result<CompressionParams, String> {
    if let Some(url) = &params.url {
        if !url.trim().is_empty() {
            return Ok(CompressionParams {
                image_url: url.trim().to_string(),
                // jpeg=1 means client wants JPEG, otherwise they want WebP (we use AVIF for WebP)
                is_webp: params.jpeg.as_ref().map(|v| v == "1").unwrap_or(false),
                is_grayscale: params.bw.as_ref().map(|v| v == "1").unwrap_or(false),
                quality: params
                    .l
                    .as_ref()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(40),
            });
        }
    }

    Err("Missing query parameters".to_string())
}

/// Compression parameters
#[derive(Debug, Clone)]
struct CompressionParams {
    image_url: String,
    is_webp: bool,
    is_grayscale: bool,
    quality: u8,
}

/// Clean and validate image URL
fn clean_image_url(url: &str) -> Result<String, String> {
    Url::parse(url.trim())
        .map(|u| u.to_string())
        .map_err(|_| "Invalid URL".to_string())
}

/// Generate MD5 hash of URL
fn generate_url_hash(url: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(url.as_bytes());
    hex::encode(hasher.finalize())
}

/// Fetch image from upstream URL
async fn fetch_upstream_image(
    url: &str,
    headers: &HeaderMap,
    client: &Client,
    config: &ServerConfig,
) -> Result<UpstreamFetchResult, String> {
    let fetch_headers = {
        let mut h = HeaderMap::new();

        // Pick relevant headers
        let picked = pick(
            &headers
                .iter()
                .filter_map(|(k, v)| v.to_str().ok().map(|vs| (k.as_str().to_string(), vs.to_string())))
                .collect(),
            &config.fetch_headers_to_pick,
        );

        for (key, value) in picked {
            if let Ok(val) = HeaderValue::from_str(&value) {
                if let Ok(name) = HeaderName::from_bytes(key.as_bytes()) {
                    h.insert(name, val);
                }
            }
        }

        h
    };

    let response = client
        .get(url)
        .headers(fetch_headers)
        .timeout(Duration::from_secs(8))
        .send()
        .await
        .map_err(|e| format!("Fetch error: {}", e))?;

    let status = response.status().as_u16();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Read error: {}", e))?;

    Ok(UpstreamFetchResult {
        status,
        content_type,
        data: bytes.to_vec(),
    })
}

/// Result of upstream fetch
struct UpstreamFetchResult {
    status: u16,
    content_type: String,
    data: Vec<u8>,
}

/// Check if compression should be bypassed
fn should_bypass_compression(
    content_length: u64,
    content_type: &str,
    is_webp: bool,
    config: &ServerConfig,
) -> Option<&'static str> {
    if content_length < config.bypass_threshold {
        return Some("already_small");
    }

    let compress_config = CompressConfig::default();
    if !should_compress(content_type, content_length, is_webp, &compress_config) {
        return Some("criteria_not_met");
    }

    if !content_type.starts_with("image/") {
        return Some("non-image");
    }

    None
}

/// Health check handler
async fn health_check() -> &'static str {
    "bandwidth-hero-proxy"
}

/// Main compression handler
async fn compress_handler(
    State(state): State<AppState>,
    Query(params): Query<CompressionQuery>,
    headers: HeaderMap,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    // Parse query parameters
    let compression_params = match parse_query_params(&params) {
        Ok(p) => p,
        Err(e) => return Err(create_error_response(StatusCode::BAD_REQUEST, &e, None)),
    };

    // Clean and validate URL
    let image_url = clean_image_url(&compression_params.image_url)
        .map_err(|e| create_error_response(StatusCode::BAD_REQUEST, &e, None))?;

    // Generate URL hash
    let url_hash = generate_url_hash(&image_url);

    // Fetch upstream image
    let fetch_result = fetch_upstream_image(
        &image_url,
        &headers,
        &state.http_client,
        &state.config,
    )
    .await
    .map_err(|e| {
        state.logger.error("Upstream fetch error", &serde_json::json!({
            "url": image_url,
            "error": e,
        }));
        create_error_response(StatusCode::BAD_GATEWAY, "Failed to fetch image", Some(image_url.clone()))
    })?;

    state.logger.log_upstream_fetch(
        &image_url,
        fetch_result.status,
        fetch_result.status >= 200 && fetch_result.status < 300,
    );

    if fetch_result.status < 200 || fetch_result.status >= 300 {
        return Err(create_error_response(
            StatusCode::BAD_GATEWAY,
            "Upstream fetch failed",
            Some(image_url),
        ));
    }

    let content_length = fetch_result.data.len() as u64;

    // Log request
    state.logger.log_request(
        &image_url,
        headers.get("user-agent").and_then(|v| v.to_str().ok()),
        headers.get("referer").and_then(|v| v.to_str().ok()),
        headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()),
        params.jpeg.as_deref(),
        params.bw.as_deref(),
        compression_params.quality,
        Some(&fetch_result.content_type),
    );

    // Check if we should bypass compression
    if let Some(reason) = should_bypass_compression(
        content_length,
        &fetch_result.content_type,
        compression_params.is_webp,
        &state.config,
    ) {
        state.logger.log_bypass(&image_url, content_length, reason);

        let mut response = create_image_response(
            fetch_result.data,
            &fetch_result.content_type,
            None,
        );
        response.headers_mut().insert(
            "x-bypass-reason",
            HeaderValue::from_str(reason).unwrap(),
        );
        response.headers_mut().insert(
            "x-url-hash",
            HeaderValue::from_str(&url_hash).unwrap(),
        );

        return Ok(response);
    }

    // Compress image
    let compression_result = compress(
        &fetch_result.data,
        !compression_params.is_webp, // use_avif = !is_webp
        compression_params.is_grayscale,
        compression_params.quality,
        content_length,
        &state.logger,
    )
    .await
    .map_err(|e| {
        state.logger.error("Compression error", &serde_json::json!({
            "url": image_url,
            "error": e.to_string(),
        }));
        create_error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Compression failed",
            Some(image_url),
        )
    })?;

    // Build response
    let content_type = format!("image/{}", compression_result.format);
    let mut response = create_image_response(
        compression_result.data,
        &content_type,
        None,
    );

    let headers = response.headers_mut();
    headers.insert(
        "x-compressed-by",
        HeaderValue::from_static("bandwidth-hero"),
    );
    headers.insert(
        "x-url-hash",
        HeaderValue::from_str(&url_hash).unwrap(),
    );
    headers.insert(
        "x-bytes-saved",
        HeaderValue::from(compression_result.bytes_saved),
    );

    Ok(response)
}

/// Create the application router
fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/api/index", get(compress_handler))
        .route("/api/index/", get(compress_handler))
        .route("/health", get(health_check))
        .route("/health/", get(health_check))
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(cors)
        .with_state(state)
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> anyhow::Result<()> {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Initialize logger
    let log_level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "INFO".to_string());
    let log_enabled = std::env::var("LOG_ENABLED").unwrap_or_else(|_| "true".to_string()) != "false";
    Logger::init(&log_level, log_enabled);

    let logger = Logger::new(&log_level, log_enabled);

    logger.info("Starting Bandwidth Hero Proxy", &serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
    }));

    // Create server configuration
    let config = ServerConfig::default();

    // Create HTTP client with optimized connection pooling
    let http_client = Client::builder()
        .timeout(Duration::from_secs(8))
        .pool_idle_timeout(Duration::from_secs(90))
        .pool_max_idle_per_host(8)
        .tcp_keepalive(Duration::from_secs(15))
        .build()
        .expect("Failed to create HTTP client");

    // Create application state
    let state = AppState {
        http_client,
        logger: logger.clone(),
        config: config.clone(),
    };

    // Create router
    let app = create_router(state);

    // Bind address
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));

    logger.info("Server listening", &serde_json::json!({
        "address": format!("0.0.0.0:{}", config.port),
    }));

    // Start server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
