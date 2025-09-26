//! # OCR Configuration Module
//!
//! This module defines configuration structures for OCR processing,
//! including recovery settings, format limits, and processing parameters.

// Constants for OCR configuration
pub const DEFAULT_LANGUAGES: &str = "eng+fra";
pub const FORMAT_DETECTION_BUFFER_SIZE: usize = 32;
pub const MIN_FORMAT_BYTES: usize = 8;
pub const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10MB limit for image files

/// Recovery configuration for error handling
#[derive(Debug, Clone)]
pub struct RecoveryConfig {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Base delay between retries in milliseconds
    pub base_retry_delay_ms: u64,
    /// Maximum delay between retries in milliseconds
    pub max_retry_delay_ms: u64,
    /// Timeout for OCR operations in seconds
    pub operation_timeout_secs: u64,
    /// Circuit breaker failure threshold
    pub circuit_breaker_threshold: u32,
    /// Circuit breaker reset timeout in seconds
    pub circuit_breaker_reset_secs: u64,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_retry_delay_ms: 1000,  // 1 second
            max_retry_delay_ms: 10000,  // 10 seconds
            operation_timeout_secs: 30, // 30 seconds
            circuit_breaker_threshold: 5,
            circuit_breaker_reset_secs: 60, // 1 minute
        }
    }
}

/// Format-specific file size limits for different image formats
#[derive(Debug, Clone)]
pub struct FormatSizeLimits {
    /// PNG format limit (higher due to better compression)
    pub png_max: u64,
    /// JPEG format limit (moderate due to lossy compression)
    pub jpeg_max: u64,
    /// BMP format limit (lower due to uncompressed nature)
    pub bmp_max: u64,
    /// TIFF format limit (can be large, multi-page support)
    pub tiff_max: u64,
    /// Minimum file size threshold for quick rejection
    pub min_quick_reject: u64,
}

impl Default for FormatSizeLimits {
    fn default() -> Self {
        Self {
            png_max: 15 * 1024 * 1024,          // 15MB for PNG
            jpeg_max: 10 * 1024 * 1024,         // 10MB for JPEG
            bmp_max: 5 * 1024 * 1024,           // 5MB for BMP
            tiff_max: 20 * 1024 * 1024,         // 20MB for TIFF
            min_quick_reject: 50 * 1024 * 1024, // 50MB quick reject
        }
    }
}

/// Configuration structure for OCR processing
#[derive(Debug, Clone)]
pub struct OcrConfig {
    /// OCR language codes (e.g., "eng", "eng+fra", "deu")
    pub languages: String,
    /// Buffer size for format detection in bytes
    pub buffer_size: usize,
    /// Minimum bytes required for format detection
    pub min_format_bytes: usize,
    /// Maximum allowed file size in bytes (general limit)
    pub max_file_size: u64,
    /// Format-specific size limits
    pub format_limits: FormatSizeLimits,
    /// Recovery and error handling configuration
    pub recovery: RecoveryConfig,
}

impl Default for OcrConfig {
    fn default() -> Self {
        Self {
            languages: DEFAULT_LANGUAGES.to_string(),
            buffer_size: FORMAT_DETECTION_BUFFER_SIZE,
            min_format_bytes: MIN_FORMAT_BYTES,
            max_file_size: MAX_FILE_SIZE,
            format_limits: FormatSizeLimits::default(),
            recovery: RecoveryConfig::default(),
        }
    }
}