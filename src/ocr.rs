//! # OCR Processing Module
//!
//! This module provides optical character recognition (OCR) functionality for extracting
//! text from images using the Tesseract OCR engine.
//!
//! ## Features
//!
//! - Text extraction from images using Tesseract OCR
//! - Automatic image format detection and validation
//! - Support for multiple languages (default: English and French)
//! - Comprehensive error handling and logging
//!
//! ## Supported Image Formats
//!
//! - PNG (Portable Network Graphics)
//! - JPEG/JPG (Joint Photographic Experts Group)
//! - BMP (Bitmap)
//! - TIFF/TIF (Tagged Image File Format)
//!
//! ## Dependencies
//!
//! - `leptess`: Rust bindings for Tesseract OCR and Leptonica
//! - `image`: Image format detection and processing
//! - `anyhow`: Error handling
//! - `log`: Logging functionality

use leptess::LepTess;
use std::fs::File;
use std::io::{BufReader, Read};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use anyhow::Result;
use log::{info, warn, error};

// Constants for OCR configuration
const DEFAULT_LANGUAGES: &str = "eng+fra";
const FORMAT_DETECTION_BUFFER_SIZE: usize = 32;
const MIN_FORMAT_BYTES: usize = 8;
const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10MB limit for image files

/// Custom error types for OCR operations
#[derive(Debug, Clone)]
pub enum OcrError {
    /// File validation errors
    ValidationError(String),
    /// OCR engine initialization errors
    InitializationError(String),
    /// Image loading errors
    ImageLoadError(String),
    /// Text extraction errors
    ExtractionError(String),
    /// Instance corruption errors
    _InstanceCorruptionError(String),
    /// Timeout errors
    TimeoutError(String),
    /// Resource exhaustion errors
    _ResourceExhaustionError(String),
}

impl std::fmt::Display for OcrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OcrError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            OcrError::InitializationError(msg) => write!(f, "Initialization error: {}", msg),
            OcrError::ImageLoadError(msg) => write!(f, "Image load error: {}", msg),
            OcrError::ExtractionError(msg) => write!(f, "Extraction error: {}", msg),
            OcrError::_InstanceCorruptionError(msg) => write!(f, "Instance corruption error: {}", msg),
            OcrError::TimeoutError(msg) => write!(f, "Timeout error: {}", msg),
            OcrError::_ResourceExhaustionError(msg) => write!(f, "Resource exhaustion error: {}", msg),
        }
    }
}

impl std::error::Error for OcrError {}

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
            base_retry_delay_ms: 1000, // 1 second
            max_retry_delay_ms: 10000, // 10 seconds
            operation_timeout_secs: 30, // 30 seconds
            circuit_breaker_threshold: 5,
            circuit_breaker_reset_secs: 60, // 1 minute
        }
    }
}

impl From<anyhow::Error> for OcrError {
    fn from(err: anyhow::Error) -> Self {
        OcrError::ExtractionError(err.to_string())
    }
}

/// Circuit breaker for OCR operations
#[derive(Debug)]
pub struct CircuitBreaker {
    failure_count: Mutex<u32>,
    last_failure_time: Mutex<Option<Instant>>,
    config: RecoveryConfig,
}

impl CircuitBreaker {
    pub fn new(config: RecoveryConfig) -> Self {
        Self {
            failure_count: Mutex::new(0),
            last_failure_time: Mutex::new(None),
            config,
        }
    }

    /// Check if circuit breaker is open
    pub fn is_open(&self) -> bool {
        let failure_count = *self.failure_count.lock().unwrap();
        let last_failure = *self.last_failure_time.lock().unwrap();

        if failure_count >= self.config.circuit_breaker_threshold {
            if let Some(last_time) = last_failure {
                let elapsed = last_time.elapsed();
                if elapsed < Duration::from_secs(self.config.circuit_breaker_reset_secs) {
                    return true; // Circuit is still open
                } else {
                    // Reset circuit breaker
                    *self.failure_count.lock().unwrap() = 0;
                    *self.last_failure_time.lock().unwrap() = None;
                }
            }
        }
        false
    }

    /// Record a failure
    pub fn record_failure(&self) {
        *self.failure_count.lock().unwrap() += 1;
        *self.last_failure_time.lock().unwrap() = Some(Instant::now());
    }

    /// Record a success
    pub fn record_success(&self) {
        *self.failure_count.lock().unwrap() = 0;
        *self.last_failure_time.lock().unwrap() = None;
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

/// Format-specific file size limits for different image formats
#[derive(Debug, Clone)]
pub struct FormatSizeLimits {
    /// PNG format limit (higher due to better compression)
    pub png_max_size: u64,
    /// JPEG format limit (moderate due to lossy compression)
    pub jpeg_max_size: u64,
    /// BMP format limit (lower due to uncompressed nature)
    pub bmp_max_size: u64,
    /// TIFF format limit (can be large, multi-page support)
    pub tiff_max_size: u64,
    /// Minimum file size threshold for quick rejection
    pub min_quick_reject_size: u64,
}

impl Default for FormatSizeLimits {
    fn default() -> Self {
        Self {
            png_max_size: 15 * 1024 * 1024,    // 15MB for PNG
            jpeg_max_size: 10 * 1024 * 1024,   // 10MB for JPEG
            bmp_max_size: 5 * 1024 * 1024,     // 5MB for BMP
            tiff_max_size: 20 * 1024 * 1024,   // 20MB for TIFF
            min_quick_reject_size: 50 * 1024 * 1024, // 50MB quick reject
        }
    }
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

/// Thread-safe OCR instance manager for reusing Tesseract instances
pub struct OcrInstanceManager {
    instances: Mutex<HashMap<String, Arc<Mutex<LepTess>>>>,
}

impl OcrInstanceManager {
    /// Create a new OCR instance manager
    pub fn new() -> Self {
        Self {
            instances: Mutex::new(HashMap::new()),
        }
    }

    /// Get or create an OCR instance for the given configuration
    pub fn get_instance(&self, config: &OcrConfig) -> Result<Arc<Mutex<LepTess>>> {
        let key = config.languages.clone();

        // Try to get existing instance
        {
            let instances = self.instances.lock().unwrap();
            if let Some(instance) = instances.get(&key) {
                return Ok(Arc::clone(instance));
            }
        }

        // Create new instance if none exists
        info!("Creating new OCR instance for languages: {}", key);
        let tess = LepTess::new(None, &key)
            .map_err(|e| anyhow::anyhow!("Failed to initialize Tesseract OCR instance: {}", e))?;

        let instance = Arc::new(Mutex::new(tess));

        // Store the instance
        {
            let mut instances = self.instances.lock().unwrap();
            instances.insert(key, Arc::clone(&instance));
        }

        Ok(instance)
    }

    /// Remove an instance (useful for cleanup or when configuration changes)
    pub fn _remove_instance(&self, languages: &str) {
        let mut instances = self.instances.lock().unwrap();
        if instances.remove(languages).is_some() {
            info!("Removed OCR instance for languages: {}", languages);
        }
    }

    /// Clear all instances (useful for memory cleanup)
    pub fn _clear_all_instances(&self) {
        let mut instances = self.instances.lock().unwrap();
        let count = instances.len();
        instances.clear();
        if count > 0 {
            info!("Cleared {} OCR instances", count);
        }
    }

    /// Get the number of cached instances
    pub fn _instance_count(&self) -> usize {
        let instances = self.instances.lock().unwrap();
        instances.len()
    }
}

impl Default for OcrInstanceManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Validate image file path and basic properties
fn validate_image_path(image_path: &str, config: &OcrConfig) -> Result<()> {
    // Check if path is provided
    if image_path.is_empty() {
        return Err(anyhow::anyhow!("Image path cannot be empty"));
    }

    // Check if file exists
    let path = std::path::Path::new(image_path);
    if !path.exists() {
        return Err(anyhow::anyhow!("Image file does not exist: {}", image_path));
    }

    // Check if it's actually a file (not a directory)
    if !path.is_file() {
        return Err(anyhow::anyhow!("Path is not a file: {}", image_path));
    }

    // Check file size
    match path.metadata() {
        Ok(metadata) => {
            let file_size = metadata.len();
            if file_size > config.max_file_size {
                return Err(anyhow::anyhow!(
                    "Image file too large: {} bytes (maximum allowed: {} bytes)",
                    file_size,
                    config.max_file_size
                ));
            }
            if file_size == 0 {
                return Err(anyhow::anyhow!("Image file is empty: {}", image_path));
            }
        }
        Err(e) => {
            return Err(anyhow::anyhow!("Cannot read file metadata: {} - {}", image_path, e));
        }
    }

    // Basic file extension check (optional but helpful)
    if let Some(extension) = path.extension() {
        let ext_str = extension.to_string_lossy().to_lowercase();
        let valid_extensions = ["png", "jpg", "jpeg", "bmp", "tiff", "tif"];
        if !valid_extensions.contains(&ext_str.as_str()) {
            info!("File extension '{}' may not be supported for OCR", ext_str);
        }
    }

    Ok(())
}

/// Enhanced validation with format-specific size limits and progressive validation
fn validate_image_with_format_limits(image_path: &str, config: &OcrConfig) -> Result<()> {
    // First, perform basic validation
    validate_image_path(image_path, config)?;

    let path = std::path::Path::new(image_path);
    let file_size = path.metadata()?.len();

    // Quick rejection for extremely large files
    if file_size > config.format_limits.min_quick_reject_size {
        info!("Quick rejecting file {}: {} bytes exceeds quick reject threshold",
              image_path, file_size);
        return Err(anyhow::anyhow!(
            "File too large for processing: {} bytes (exceeds quick reject threshold of {} bytes)",
            file_size, config.format_limits.min_quick_reject_size
        ));
    }

    // Try to detect format and apply format-specific limits
    match File::open(image_path) {
        Ok(file) => {
            let mut reader = BufReader::new(file);
            let mut buffer = vec![0; config.buffer_size];

            match reader.read(&mut buffer) {
                Ok(bytes_read) if bytes_read >= config.min_format_bytes => {
                    buffer.truncate(bytes_read);

                    match image::guess_format(&buffer) {
                        Ok(format) => {
                            let format_limit = match format {
                                image::ImageFormat::Png => {
                                    info!("Detected PNG format for {}, applying {}MB limit",
                                          image_path, config.format_limits.png_max_size / (1024 * 1024));
                                    config.format_limits.png_max_size
                                }
                                image::ImageFormat::Jpeg => {
                                    info!("Detected JPEG format for {}, applying {}MB limit",
                                          image_path, config.format_limits.jpeg_max_size / (1024 * 1024));
                                    config.format_limits.jpeg_max_size
                                }
                                image::ImageFormat::Bmp => {
                                    info!("Detected BMP format for {}, applying {}MB limit",
                                          image_path, config.format_limits.bmp_max_size / (1024 * 1024));
                                    config.format_limits.bmp_max_size
                                }
                                image::ImageFormat::Tiff => {
                                    info!("Detected TIFF format for {}, applying {}MB limit",
                                          image_path, config.format_limits.tiff_max_size / (1024 * 1024));
                                    config.format_limits.tiff_max_size
                                }
                                _ => {
                                    info!("Detected unsupported format {:?} for {}, using general limit",
                                          format, image_path);
                                    config.max_file_size
                                }
                            };

                            if file_size > format_limit {
                                return Err(anyhow::anyhow!(
                                    "Image file too large for {:?} format: {} bytes (maximum allowed: {} bytes)",
                                    format, file_size, format_limit
                                ));
                            }

                            // Estimate memory usage for processing
                            let estimated_memory_mb = estimate_memory_usage(file_size, &format);
                            info!("Estimated memory usage for {}: {}MB", image_path, estimated_memory_mb);

                            Ok(())
                        }
                        Err(_) => {
                            // Could not determine format, use general limit
                            info!("Could not determine image format for {}, using general size limit", image_path);
                            if file_size > config.max_file_size {
                                return Err(anyhow::anyhow!(
                                    "Image file too large: {} bytes (maximum allowed: {} bytes)",
                                    file_size, config.max_file_size
                                ));
                            }
                            Ok(())
                        }
                    }
                }
                _ => {
                    // Could not read enough bytes, use general limit
                    info!("Could not read enough bytes for format detection from {}, using general size limit", image_path);
                    if file_size > config.max_file_size {
                        return Err(anyhow::anyhow!(
                            "Image file too large: {} bytes (maximum allowed: {} bytes)",
                            file_size, config.max_file_size
                        ));
                    }
                    Ok(())
                }
            }
        }
        Err(e) => {
            return Err(anyhow::anyhow!("Cannot open image file for validation: {} - {}", image_path, e));
        }
    }
}

/// Estimate memory usage for image processing based on file size and format
fn estimate_memory_usage(file_size: u64, format: &image::ImageFormat) -> f64 {
    let file_size_mb = file_size as f64 / (1024.0 * 1024.0);

    // Memory estimation factors based on format characteristics
    let memory_factor = match format {
        image::ImageFormat::Png => 3.0,   // PNG decompression can use 2-4x file size
        image::ImageFormat::Jpeg => 2.5,  // JPEG decompression uses ~2-3x
        image::ImageFormat::Bmp => 1.2,   // BMP is mostly uncompressed
        image::ImageFormat::Tiff => 4.0,  // TIFF can be complex with layers
        _ => 3.0, // Default estimation
    };

    file_size_mb * memory_factor
}

/// Extract text from an image using Tesseract OCR with instance reuse
pub async fn extract_text_from_image(image_path: &str, config: &OcrConfig, instance_manager: &OcrInstanceManager) -> Result<String, OcrError> {
    // Validate input with enhanced format-specific validation
    validate_image_with_format_limits(image_path, config)
        .map_err(|e| OcrError::ValidationError(e.to_string()))?;

    info!("Starting OCR text extraction from image: {}", image_path);

    // Implement retry logic with exponential backoff
    let mut attempt = 0;
    let max_attempts = config.recovery.max_retries + 1; // +1 for initial attempt

    loop {
        attempt += 1;

        match perform_ocr_extraction(image_path, config, instance_manager).await {
            Ok(text) => {
                info!("OCR extraction completed successfully on attempt {}. Extracted {} characters of text",
                      attempt, text.len());
                return Ok(text);
            }
            Err(err) => {
                if attempt >= max_attempts {
                    error!("OCR extraction failed after {} attempts: {:?}", max_attempts, err);
                    return Err(err);
                }

                let delay_ms = calculate_retry_delay(attempt, &config.recovery);
                warn!("OCR extraction attempt {} failed: {:?}. Retrying in {}ms", attempt, err, delay_ms);

                tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
            }
        }
    }
}

/// Helper function to perform OCR extraction with timeout
async fn perform_ocr_extraction(image_path: &str, config: &OcrConfig, instance_manager: &OcrInstanceManager) -> Result<String, OcrError> {
    // Create a timeout for the operation
    let timeout_duration = tokio::time::Duration::from_secs(config.recovery.operation_timeout_secs);

    tokio::time::timeout(timeout_duration, async {
        // Get or create OCR instance from the manager
        let instance = instance_manager.get_instance(config)
            .map_err(|e| OcrError::InitializationError(e.to_string()))?;

        // Perform OCR processing with the reused instance
        let extracted_text = {
            let mut tess = instance.lock().unwrap();
            // Set the image for OCR processing
            tess.set_image(image_path)
                .map_err(|e| OcrError::ImageLoadError(format!("Failed to load image for OCR: {}", e)))?;

            // Extract text from the image
            tess.get_utf8_text()
                .map_err(|e| OcrError::ExtractionError(format!("Failed to extract text from image: {}", e)))?
        };

        // Clean up the extracted text (remove extra whitespace and empty lines)
        let cleaned_text = extracted_text
            .trim()
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<&str>>()
            .join("\n");

        Ok(cleaned_text)
    })
    .await
    .map_err(|_| OcrError::TimeoutError(format!("OCR operation timed out after {} seconds", config.recovery.operation_timeout_secs)))?
}

/// Calculate retry delay with exponential backoff
fn calculate_retry_delay(attempt: u32, recovery: &RecoveryConfig) -> u64 {
    let base_delay = recovery.base_retry_delay_ms as f64;
    let exponential_delay = base_delay * (2.0_f64).powf((attempt - 1) as f64);
    let delay = exponential_delay.min(recovery.max_retry_delay_ms as f64) as u64;

    // Add some jitter to prevent thundering herd
    let jitter = (rand::random::<u64>() % (delay / 4)) as u64;
    delay + jitter
}

/// Validate if an image file is supported for OCR processing using image::guess_format
pub fn is_supported_image_format(file_path: &str, config: &OcrConfig) -> bool {
    // Enhanced validation first (includes size checks)
    if let Err(_) = validate_image_with_format_limits(file_path, config) {
        return false;
    }

    match File::open(file_path) {
        Ok(file) => {
            let mut reader = BufReader::new(file);
            let mut buffer = vec![0; config.buffer_size]; // Pre-allocate buffer for format detection

            match reader.read(&mut buffer) {
                Ok(bytes_read) if bytes_read >= config.min_format_bytes => {
                    // Truncate buffer to actual bytes read
                    buffer.truncate(bytes_read);

                    info!("Read {} bytes from file {} for format detection", bytes_read, file_path);

                    match image::guess_format(&buffer) {
                        Ok(format) => {
                            // Tesseract supports: PNG, JPEG/JPG, BMP, TIFF
                            let supported = matches!(
                                format,
                                image::ImageFormat::Png |
                                image::ImageFormat::Jpeg |
                                image::ImageFormat::Bmp |
                                image::ImageFormat::Tiff
                            );

                            if supported {
                                info!("Detected supported image format: {:?} for file: {}", format, file_path);
                            } else {
                                info!("Detected unsupported image format: {:?} for file: {}", format, file_path);
                            }

                            supported
                        }
                        Err(e) => {
                            info!("Could not determine image format for file: {} - {}", file_path, e);
                            false
                        }
                    }
                }
                Ok(bytes_read) => {
                    info!("Could not read enough bytes to determine image format for file: {} (read {} bytes, need at least {})", file_path, bytes_read, config.min_format_bytes);
                    false
                }
                Err(e) => {
                    info!("Error reading image file for format detection: {} - {}", file_path, e);
                    false
                }
            }
        }
        Err(e) => {
            info!("Could not open image file for format detection: {} - {}", file_path, e);
            false
        }
    }
}
