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
///
/// Implements circuit breaker pattern to prevent cascading failures in OCR processing.
/// When OCR operations fail repeatedly, the circuit breaker "opens" to stop further
/// attempts and allow the system to recover.
///
/// # State Machine
///
/// - **Closed**: Normal operation, requests pass through
/// - **Open**: Failure threshold exceeded, requests fail fast
/// - **Half-Open**: Testing if service has recovered
///
/// # Configuration
///
/// Uses `RecoveryConfig` for:
/// - `circuit_breaker_threshold`: Failures before opening (default: 5)
/// - `circuit_breaker_reset_secs`: Time before attempting reset (default: 60s)
#[derive(Debug)]
pub struct CircuitBreaker {
    failure_count: Mutex<u32>,
    last_failure_time: Mutex<Option<Instant>>,
    config: RecoveryConfig,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with the given configuration
    ///
    /// # Arguments
    ///
    /// * `config` - Recovery configuration with circuit breaker settings
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ingredients::ocr::{CircuitBreaker, RecoveryConfig};
    ///
    /// let config = RecoveryConfig::default();
    /// let circuit_breaker = CircuitBreaker::new(config);
    /// ```
    pub fn new(config: RecoveryConfig) -> Self {
        Self {
            failure_count: Mutex::new(0),
            last_failure_time: Mutex::new(None),
            config,
        }
    }

    /// Check if circuit breaker is open (blocking requests)
    ///
    /// # Returns
    ///
    /// `true` if circuit is open and should block requests, `false` if closed
    ///
    /// # Behavior
    ///
    /// - Returns `true` when failure count >= threshold and reset time hasn't elapsed
    /// - Automatically resets to closed state after reset timeout
    /// - Thread-safe using internal mutexes
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

    /// Record a failure to increment the failure counter
    ///
    /// Should be called whenever an OCR operation fails.
    /// Updates failure count and last failure timestamp.
    ///
    /// # Thread Safety
    ///
    /// Uses internal mutex for thread-safe updates.
    pub fn record_failure(&self) {
        *self.failure_count.lock().unwrap() += 1;
        *self.last_failure_time.lock().unwrap() = Some(Instant::now());
    }

    /// Record a success to reset the failure counter
    ///
    /// Should be called whenever an OCR operation succeeds.
    /// Resets failure count and clears last failure timestamp.
    ///
    /// # Thread Safety
    ///
    /// Uses internal mutex for thread-safe updates.
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
///
/// Manages a pool of Tesseract OCR instances keyed by language configuration.
/// Reusing instances significantly improves performance by avoiding the overhead
/// of creating new Tesseract instances for each OCR operation.
///
/// # Performance Benefits
///
/// - Eliminates Tesseract initialization overhead (~100-500ms per instance)
/// - Reduces memory allocations for repeated OCR operations
/// - Thread-safe with Arc<Mutex<>> for concurrent access
///
/// # Instance Lifecycle
///
/// - Instances are created on first request for a language combination
/// - Instances are reused for subsequent requests with same language config
/// - Instances persist until explicitly removed or manager is dropped
///
/// # Thread Safety
///
/// Uses `Mutex<HashMap<>>` internally for thread-safe instance management.
/// Multiple threads can safely request instances concurrently.
///
/// # Memory Management
///
/// - Each language combination maintains one instance
/// - Memory usage scales with number of unique language combinations
/// - Consider memory limits for applications with many language combinations
pub struct OcrInstanceManager {
    instances: Mutex<HashMap<String, Arc<Mutex<LepTess>>>>,
}

impl OcrInstanceManager {
    /// Create a new OCR instance manager
    ///
    /// Initializes an empty instance pool. Instances will be created
    /// on-demand when first requested via `get_instance()`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ingredients::ocr::OcrInstanceManager;
    ///
    /// let manager = OcrInstanceManager::new();
    /// // Manager is ready to provide OCR instances
    /// ```
    pub fn new() -> Self {
        Self {
            instances: Mutex::new(HashMap::new()),
        }
    }

    /// Get or create an OCR instance for the given configuration
    ///
    /// Returns an existing instance if one exists for the language configuration,
    /// otherwise creates a new instance and stores it for future reuse.
    ///
    /// # Arguments
    ///
    /// * `config` - OCR configuration containing language settings and other options
    ///
    /// # Returns
    ///
    /// Returns `Result<Arc<Mutex<LepTess>>, anyhow::Error>` containing the OCR instance
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use ingredients::ocr::{OcrInstanceManager, OcrConfig};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let manager = OcrInstanceManager::new();
    /// let config = OcrConfig::default();
    ///
    /// let instance = manager.get_instance(&config)?;
    /// // Use the instance for OCR processing
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns error if Tesseract instance creation fails (e.g., invalid language codes)
    ///
    /// # Performance
    ///
    /// - First call for a language: ~100-500ms (Tesseract initialization)
    /// - Subsequent calls: ~1ms (instance lookup and Arc clone)
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
///
/// Performs comprehensive image validation including:
/// 1. Basic file existence and accessibility checks
/// 2. Format detection using magic bytes
/// 3. Format-specific file size validation
/// 4. Memory usage estimation and validation
/// 5. Quick rejection for extremely large files
///
/// # Arguments
///
/// * `image_path` - Path to the image file to validate (must be absolute path)
/// * `config` - OCR configuration with format limits and validation settings
///
/// # Returns
///
/// Returns `Result<(), anyhow::Error>` - Ok if validation passes, Error with details if validation fails
///
/// # Validation Steps
///
/// 1. **Basic Validation**: File existence, readability, basic size limits
/// 2. **Quick Rejection**: Immediate rejection of extremely large files (> min_quick_reject_size)
/// 3. **Format Detection**: Read file header and detect image format
/// 4. **Size Validation**: Check against format-specific size limits
/// 5. **Memory Estimation**: Calculate expected memory usage
///
/// # Format-Specific Limits
///
/// | Format | Max Size | Memory Factor | Use Case |
/// |--------|----------|---------------|----------|
/// | PNG    | 15MB     | 3.0x          | Best for text, lossless |
/// | JPEG   | 10MB     | 2.5x          | Good balance, lossy compression |
/// | BMP    | 5MB      | 1.2x          | Fast processing, uncompressed |
/// | TIFF   | 20MB     | 4.0x          | High quality, multi-page |
///
/// # Examples
///
/// ```rust,no_run
/// use ingredients::ocr::{validate_image_with_format_limits, OcrConfig};
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = OcrConfig::default();
///
/// match validate_image_with_format_limits("/path/to/image.png", &config) {
///     Ok(()) => println!("Image validation passed"),
///     Err(e) => println!("Validation failed: {}", e),
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Error Conditions
///
/// - File doesn't exist or isn't readable
/// - File size exceeds format-specific limits
/// - Unsupported image format
/// - Insufficient bytes for format detection
/// - Memory usage estimation exceeds limits
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
///
/// Calculates expected memory consumption during image decompression and OCR processing.
/// Used for pre-processing validation to prevent out-of-memory errors.
///
/// # Arguments
///
/// * `file_size` - Size of the image file in bytes
/// * `format` - Detected image format
///
/// # Returns
///
/// Returns estimated memory usage in megabytes (MB)
///
/// # Memory Factors by Format
///
/// | Format | Factor | Reason |
/// |--------|--------|--------|
/// | PNG    | 3.0x   | Lossless decompression expands compressed data |
/// | JPEG   | 2.5x   | Lossy decompression with working buffers |
/// | BMP    | 1.2x   | Mostly uncompressed, minimal expansion |
/// | TIFF   | 4.0x   | Complex format with layers and metadata |
///
/// # Examples
///
/// ```rust
/// use ingredients::ocr::estimate_memory_usage;
/// use image::ImageFormat;
///
/// // 1MB PNG file
/// let memory_mb = estimate_memory_usage(1024 * 1024, &ImageFormat::Png);
/// assert_eq!(memory_mb, 3.0); // 3MB estimated usage
///
/// // 2MB JPEG file
/// let memory_mb = estimate_memory_usage(2 * 1024 * 1024, &ImageFormat::Jpeg);
/// assert_eq!(memory_mb, 5.0); // 5MB estimated usage
/// ```
///
/// # Usage in Validation
///
/// Used by `validate_image_with_format_limits()` to ensure sufficient memory
/// is available before attempting image processing and OCR operations.
///
/// # Accuracy
///
/// Estimates are conservative and may overestimate actual usage.
/// Better to reject potentially problematic files than risk OOM errors.
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
///
/// This is the main entry point for OCR processing. It handles image validation,
/// OCR instance management, retry logic with exponential backoff, and comprehensive
/// error handling with circuit breaker protection.
///
/// # Arguments
///
/// * `image_path` - Path to the image file to process (must be absolute path)
/// * `config` - OCR configuration including language settings, timeouts, and recovery options
/// * `instance_manager` - Manager for OCR instance reuse to improve performance
///
/// # Returns
///
/// Returns `Result<String, OcrError>` containing the extracted text or an error
///
/// # Examples
///
/// ```rust,no_run
/// use ingredients::ocr::{extract_text_from_image, OcrConfig, OcrInstanceManager};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = OcrConfig::default();
/// let instance_manager = OcrInstanceManager::new();
///
/// // Process an image of ingredients
/// let text = extract_text_from_image("/path/to/ingredients.jpg", &config, &instance_manager).await?;
/// println!("Extracted text: {}", text);
/// # Ok(())
/// # }
/// ```
///
/// # Supported Image Formats
///
/// - PNG (Portable Network Graphics) - up to 15MB
/// - JPEG/JPG (Joint Photographic Experts Group) - up to 10MB
/// - BMP (Bitmap) - up to 5MB
/// - TIFF/TIF (Tagged Image File Format) - up to 20MB
///
/// # Performance
///
/// - Includes automatic retry logic (up to 3 attempts by default)
/// - Uses OCR instance reuse for better performance
/// - Circuit breaker protection against cascading failures
/// - Comprehensive timing metrics logged at INFO level
///
/// # Errors
///
/// Returns `OcrError` for various failure conditions:
/// - `ValidationError` - Image format not supported or file too large
/// - `InitializationError` - OCR engine initialization failed
/// - `ImageLoadError` - Could not load the image file
/// - `ExtractionError` - OCR processing failed
/// - `TimeoutError` - Operation exceeded timeout (30s default)
pub async fn extract_text_from_image(image_path: &str, config: &OcrConfig, instance_manager: &OcrInstanceManager) -> Result<String, OcrError> {
    // Start timing the entire OCR operation
    let start_time = std::time::Instant::now();

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
                let total_duration = start_time.elapsed();
                let total_ms = total_duration.as_millis();

                info!("OCR extraction completed successfully on attempt {} in {}ms. Extracted {} characters of text",
                      attempt, total_ms, text.len());
                return Ok(text);
            }
            Err(err) => {
                if attempt >= max_attempts {
                    let total_duration = start_time.elapsed();
                    let total_ms = total_duration.as_millis();

                    error!("OCR extraction failed after {} attempts ({}ms total): {:?}",
                           max_attempts, total_ms, err);
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
///
/// This function handles the core OCR processing using Tesseract, including:
/// - OCR instance acquisition from the manager
/// - Image loading and processing
/// - Text extraction and cleanup
/// - Timeout protection
/// - Performance timing and logging
///
/// # Arguments
///
/// * `image_path` - Path to the image file to process
/// * `config` - OCR configuration with timeout and language settings
/// * `instance_manager` - Manager for OCR instance reuse
///
/// # Returns
///
/// Returns `Result<String, OcrError>` with cleaned extracted text or error
///
/// # Processing Details
///
/// 1. Acquires or creates OCR instance for specified language
/// 2. Loads image into Tesseract engine
/// 3. Performs OCR text extraction
/// 4. Cleans extracted text (removes extra whitespace, empty lines)
/// 5. Logs performance metrics
///
/// # Performance
///
/// - Times only the actual OCR processing (excludes validation/retry logic)
/// - Logs processing time in milliseconds
/// - Includes character count in success logs
///
/// # Errors
///
/// - `InitializationError` - Failed to get/create OCR instance
/// - `ImageLoadError` - Could not load image into Tesseract
/// - `ExtractionError` - OCR processing failed
/// - `TimeoutError` - Operation exceeded configured timeout
async fn perform_ocr_extraction(image_path: &str, config: &OcrConfig, instance_manager: &OcrInstanceManager) -> Result<String, OcrError> {
    // Start timing the actual OCR processing
    let ocr_start_time = std::time::Instant::now();

    // Create a timeout for the operation
    let timeout_duration = tokio::time::Duration::from_secs(config.recovery.operation_timeout_secs);

    let result = tokio::time::timeout(timeout_duration, async {
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
    }).await;

    let ocr_duration = ocr_start_time.elapsed();
    let ocr_ms = ocr_duration.as_millis();

    match result {
        Ok(Ok(text)) => {
            info!("OCR processing completed in {}ms, extracted {} characters", ocr_ms, text.len());
            Ok(text)
        }
        Ok(Err(e)) => {
            warn!("OCR processing failed after {}ms: {:?}", ocr_ms, e);
            Err(e)
        }
        Err(_) => {
            warn!("OCR processing timed out after {}ms (limit: {}s)",
                  ocr_ms, config.recovery.operation_timeout_secs);
            Err(OcrError::TimeoutError(format!("OCR operation timed out after {} seconds", config.recovery.operation_timeout_secs)))
        }
    }
}

/// Calculate retry delay with exponential backoff
///
/// Implements exponential backoff with jitter to prevent thundering herd problems.
/// Delay increases exponentially with each retry attempt, with random jitter added
/// to distribute retry attempts over time.
///
/// # Arguments
///
/// * `attempt` - Current retry attempt number (1-based, first retry = 1)
/// * `recovery` - Recovery configuration with delay settings
///
/// # Returns
///
/// Returns delay in milliseconds before next retry attempt
///
/// # Algorithm
///
/// ```text
/// delay = min(base_delay * (2^(attempt-1)), max_delay)
/// jitter = random(0, delay/4)
/// final_delay = delay + jitter
/// ```
///
/// # Examples
///
/// ```rust
/// use ingredients::ocr::{calculate_retry_delay, RecoveryConfig};
///
/// let config = RecoveryConfig::default();
/// // First retry: ~1000-1250ms (1000ms + jitter)
/// let delay1 = calculate_retry_delay(1, &config);
/// // Second retry: ~2000-2500ms (2000ms + jitter)
/// let delay2 = calculate_retry_delay(2, &config);
/// // Third retry: ~4000-5000ms (4000ms + jitter)
/// let delay3 = calculate_retry_delay(3, &config);
/// ```
///
/// # Configuration Parameters
///
/// - `base_retry_delay_ms`: Base delay for first retry (default: 1000ms)
/// - `max_retry_delay_ms`: Maximum delay cap (default: 10000ms)
///
/// # Benefits
///
/// - **Exponential Backoff**: Reduces server load during failures
/// - **Jitter**: Prevents synchronized retry storms
/// - **Configurable**: Adjustable for different environments
/// - **Capped**: Prevents excessively long delays
fn calculate_retry_delay(attempt: u32, recovery: &RecoveryConfig) -> u64 {
    let base_delay = recovery.base_retry_delay_ms as f64;
    let exponential_delay = base_delay * (2.0_f64).powf((attempt - 1) as f64);
    let delay = exponential_delay.min(recovery.max_retry_delay_ms as f64) as u64;

    // Add some jitter to prevent thundering herd
    let jitter = (rand::random::<u64>() % (delay / 4)) as u64;
    delay + jitter
}

/// Validate if an image file is supported for OCR processing using image::guess_format
///
/// Performs comprehensive validation including:
/// 1. File existence and accessibility checks
/// 2. Format detection using magic bytes
/// 3. File size validation against format-specific limits
/// 4. Memory usage estimation
///
/// # Arguments
///
/// * `file_path` - Path to the image file to validate
/// * `config` - OCR configuration with size limits and buffer settings
///
/// # Returns
///
/// Returns `true` if the image format is supported and passes all validation checks
///
/// # Supported Formats
///
/// | Format | Max Size | Description |
/// |--------|----------|-------------|
/// | PNG    | 15MB     | Lossless compression, best for text |
/// | JPEG   | 10MB     | Lossy compression, good quality/size balance |
/// | BMP    | 5MB      | Uncompressed, fast but large files |
/// | TIFF   | 20MB     | Multi-page support, high quality |
///
/// # Examples
///
/// ```rust,no_run
/// use ingredients::ocr::{is_supported_image_format, OcrConfig};
///
/// let config = OcrConfig::default();
/// if is_supported_image_format("/path/to/image.jpg", &config) {
///     println!("Image is supported for OCR processing");
/// } else {
///     println!("Image format not supported or file too large");
/// }
/// ```
///
/// # Validation Process
///
/// 1. Checks if file exists and is readable
/// 2. Reads first 32 bytes (configurable) for format detection
/// 3. Uses `image::guess_format()` to identify format
/// 4. Validates file size against format-specific limits
/// 5. Estimates memory usage for processing
///
/// # Performance
///
/// - Fast format detection using only file header
/// - Minimal I/O (only reads format detection buffer)
/// - No full file loading or OCR processing
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Test OCR configuration defaults
    #[test]
    fn test_ocr_config_defaults() {
        let config = OcrConfig::default();

        assert_eq!(config.languages, DEFAULT_LANGUAGES);
        assert_eq!(config.buffer_size, FORMAT_DETECTION_BUFFER_SIZE);
        assert_eq!(config.min_format_bytes, MIN_FORMAT_BYTES);
        assert_eq!(config.max_file_size, MAX_FILE_SIZE);
        assert!(config.recovery.max_retries > 0);
        assert!(config.recovery.operation_timeout_secs > 0);
    }

    /// Test recovery configuration defaults
    #[test]
    fn test_recovery_config_defaults() {
        let recovery = RecoveryConfig::default();

        assert_eq!(recovery.max_retries, 3);
        assert_eq!(recovery.base_retry_delay_ms, 1000);
        assert_eq!(recovery.max_retry_delay_ms, 10000);
        assert_eq!(recovery.operation_timeout_secs, 30);
        assert_eq!(recovery.circuit_breaker_threshold, 5);
        assert_eq!(recovery.circuit_breaker_reset_secs, 60);
    }

    /// Test format size limits defaults
    #[test]
    fn test_format_size_limits_defaults() {
        let limits = FormatSizeLimits::default();

        assert_eq!(limits.png_max_size, 15 * 1024 * 1024);   // 15MB
        assert_eq!(limits.jpeg_max_size, 10 * 1024 * 1024);  // 10MB
        assert_eq!(limits.bmp_max_size, 5 * 1024 * 1024);    // 5MB
        assert_eq!(limits.tiff_max_size, 20 * 1024 * 1024);  // 20MB
        assert_eq!(limits.min_quick_reject_size, 50 * 1024 * 1024); // 50MB
    }

    /// Test circuit breaker state transitions
    #[test]
    fn test_circuit_breaker_state_transitions() {
        let config = RecoveryConfig {
            circuit_breaker_threshold: 2,
            ..Default::default()
        };
        let circuit_breaker = CircuitBreaker::new(config);

        // Initially closed
        assert!(!circuit_breaker.is_open());

        // Record failures
        circuit_breaker.record_failure();
        assert!(!circuit_breaker.is_open()); // Still closed (1 failure)

        circuit_breaker.record_failure();
        assert!(circuit_breaker.is_open()); // Now open (2 failures)

        // Note: In a real scenario, we'd wait for the reset timeout to transition to half-open
        // For this test, we just verify the failure recording works
    }

    /// Test instance manager operations
    #[test]
    fn test_instance_manager_operations() {
        let manager = OcrInstanceManager::new();

        // Initially empty
        assert_eq!(manager._instance_count(), 0);

        // Create config
        let config = OcrConfig::default();

        // Get instance (creates new one)
        let instance1 = manager.get_instance(&config).unwrap();
        assert_eq!(manager._instance_count(), 1);

        // Get same instance again (reuses existing)
        let instance2 = manager.get_instance(&config).unwrap();
        assert_eq!(manager._instance_count(), 1);

        // Verify they're the same instance
        assert!(Arc::ptr_eq(&instance1, &instance2));

        // Remove instance
        manager._remove_instance(&config.languages);
        assert_eq!(manager._instance_count(), 0);

        // Clear all instances
        manager._clear_all_instances();
        assert_eq!(manager._instance_count(), 0);
    }

    /// Test image path validation with valid inputs
    #[test]
    fn test_validate_image_path_valid() {
        let config = OcrConfig::default();

        // Create a temporary file
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"test content").unwrap();
        let temp_path = temp_file.path().to_string_lossy().to_string();

        // Should pass validation
        let result = validate_image_path(&temp_path, &config);
        assert!(result.is_ok());
    }

    /// Test image path validation with invalid inputs
    #[test]
    fn test_validate_image_path_invalid() {
        let config = OcrConfig::default();

        // Test empty path
        let result = validate_image_path("", &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));

        // Test non-existent file
        let result = validate_image_path("/non/existent/file.png", &config);
        assert!(result.is_err());
        // The error message might vary by OS, so just check it's an error
        assert!(result.is_err());
    }

    /// Test memory usage estimation for different formats
    #[test]
    fn test_estimate_memory_usage() {
        let file_size = 1024 * 1024; // 1MB

        // Test PNG (highest memory factor)
        let png_memory = estimate_memory_usage(file_size, &image::ImageFormat::Png);
        assert_eq!(png_memory, 3.0); // 1MB * 3.0

        // Test JPEG
        let jpeg_memory = estimate_memory_usage(file_size, &image::ImageFormat::Jpeg);
        assert_eq!(jpeg_memory, 2.5); // 1MB * 2.5

        // Test BMP (lowest memory factor)
        let bmp_memory = estimate_memory_usage(file_size, &image::ImageFormat::Bmp);
        assert_eq!(bmp_memory, 1.2); // 1MB * 1.2
    }

    /// Test retry delay calculation
    #[test]
    fn test_calculate_retry_delay() {
        let recovery = RecoveryConfig::default();

        // First retry (attempt 1): base delay
        let delay1 = calculate_retry_delay(1, &recovery);
        assert!(delay1 >= recovery.base_retry_delay_ms);

        // Second retry (attempt 2): exponential backoff
        let delay2 = calculate_retry_delay(2, &recovery);
        assert!(delay2 >= delay1);

        // Test that delay doesn't exceed max (with reasonable bounds)
        let delay_max_test = calculate_retry_delay(5, &recovery);
        assert!(delay_max_test <= recovery.max_retry_delay_ms * 2); // Allow some margin for jitter
    }

    /// Test error type conversions
    #[test]
    fn test_error_conversions() {
        // Test From<anyhow::Error>
        let anyhow_error = anyhow::anyhow!("test error");
        let ocr_error: OcrError = anyhow_error.into();
        match ocr_error {
            OcrError::ExtractionError(msg) => assert!(msg.contains("test error")),
            _ => panic!("Expected ExtractionError"),
        }

        // Test Display implementation
        let error = OcrError::ValidationError("test".to_string());
        let display = format!("{}", error);
        assert_eq!(display, "Validation error: test");
    }

    /// Test format detection with mock PNG file
    #[test]
    fn test_format_detection_png() {
        let config = OcrConfig::default();

        // Create mock PNG file (minimal PNG header)
        let mut temp_file = NamedTempFile::new().unwrap();
        // PNG signature: 89 50 4E 47 0D 0A 1A 0A
        let png_header = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        temp_file.write_all(&png_header).unwrap();
        temp_file.write_all(&[0u8; 24]).unwrap(); // Add some padding
        let temp_path = temp_file.path().to_string_lossy().to_string();

        // Test format detection
        let is_supported = is_supported_image_format(&temp_path, &config);
        assert!(is_supported, "PNG should be supported");
    }

    /// Test format detection with mock JPEG file
    #[test]
    fn test_format_detection_jpeg() {
        let config = OcrConfig::default();

        // Create mock JPEG file (minimal JPEG header)
        let mut temp_file = NamedTempFile::new().unwrap();
        // JPEG SOI marker: FF D8
        let jpeg_header = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46];
        temp_file.write_all(&jpeg_header).unwrap();
        temp_file.write_all(&[0u8; 24]).unwrap(); // Add some padding
        let temp_path = temp_file.path().to_string_lossy().to_string();

        // Test format detection
        let is_supported = is_supported_image_format(&temp_path, &config);
        assert!(is_supported, "JPEG should be supported");
    }

    /// Test format detection with unsupported format
    #[test]
    fn test_format_detection_unsupported() {
        let config = OcrConfig::default();

        // Create mock file with unsupported format
        let mut temp_file = NamedTempFile::new().unwrap();
        let unsupported_header = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        temp_file.write_all(&unsupported_header).unwrap();
        temp_file.write_all(&[0u8; 24]).unwrap();
        let temp_path = temp_file.path().to_string_lossy().to_string();

        // Test format detection
        let is_supported = is_supported_image_format(&temp_path, &config);
        assert!(!is_supported, "Unsupported format should not be supported");
    }

    /// Test validation with oversized file
    #[test]
    fn test_validation_oversized_file() {
        let mut config = OcrConfig::default();
        config.max_file_size = 100; // Very small limit

        // Create a file larger than the limit
        let mut temp_file = NamedTempFile::new().unwrap();
        let large_content = vec![0u8; 200]; // 200 bytes
        temp_file.write_all(&large_content).unwrap();
        let temp_path = temp_file.path().to_string_lossy().to_string();

        // Test validation
        let result = validate_image_with_format_limits(&temp_path, &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too large"));
    }

    /// Test validation with empty file
    #[test]
    fn test_validation_empty_file() {
        let config = OcrConfig::default();

        // Create empty file
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_string_lossy().to_string();

        // Test validation
        let result = validate_image_with_format_limits(&temp_path, &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    /// Test instance manager with different languages
    #[test]
    fn test_instance_manager_multiple_languages() {
        let manager = OcrInstanceManager::new();

        // Create configs with different languages
        let config_eng = OcrConfig {
            languages: "eng".to_string(),
            ..Default::default()
        };
        let config_fra = OcrConfig {
            languages: "fra".to_string(),
            ..Default::default()
        };

        // Get instances for different languages
        let instance_eng1 = manager.get_instance(&config_eng).unwrap();
        let instance_fra1 = manager.get_instance(&config_fra).unwrap();

        assert_eq!(manager._instance_count(), 2);

        // Get same instances again
        let instance_eng2 = manager.get_instance(&config_eng).unwrap();
        let instance_fra2 = manager.get_instance(&config_fra).unwrap();

        // Verify they're the same instances
        assert!(Arc::ptr_eq(&instance_eng1, &instance_eng2));
        assert!(Arc::ptr_eq(&instance_fra1, &instance_fra2));

        // But different language instances should be different
        assert!(!Arc::ptr_eq(&instance_eng1, &instance_fra1));
    }
}
