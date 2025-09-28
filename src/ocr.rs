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

use anyhow::Result;
use std::fs::File;
use std::io::{BufReader, Read};
use tracing::{debug, error, info, warn};

// Re-export types for easier access from documentation and external usage
pub use crate::circuit_breaker::CircuitBreaker;
pub use crate::instance_manager::OcrInstanceManager;
pub use crate::ocr_config::{OcrConfig, RecoveryConfig};
pub use crate::ocr_errors::OcrError;

/// Validate image file path and basic properties
pub fn validate_image_path(image_path: &str, config: &crate::ocr_config::OcrConfig) -> Result<()> {
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
            return Err(anyhow::anyhow!(
                "Cannot read file metadata: {} - {}",
                image_path,
                e
            ));
        }
    }

    // Basic file extension check (optional but helpful)
    if let Some(extension) = path.extension() {
        let ext_str = extension.to_string_lossy().to_lowercase();
        let valid_extensions = ["png", "jpg", "jpeg", "bmp", "tiff", "tif"];
        if !valid_extensions.contains(&ext_str.as_str()) {
            info!("File extension '{ext_str}' may not be supported for OCR");
        }
    }

    Ok(())
}

/// Enhanced validation with format-specific size limits and progressive validation
pub fn validate_image_with_format_limits(
    image_path: &str,
    config: &crate::ocr_config::OcrConfig,
) -> Result<()> {
    // First, perform basic validation
    validate_image_path(image_path, config)?;

    let path = std::path::Path::new(image_path);
    let file_size = path.metadata()?.len();

    // Quick rejection for extremely large files
    if file_size > config.format_limits.min_quick_reject {
        info!(
            "Quick rejecting file {image_path}: {file_size} bytes exceeds quick reject threshold"
        );
        return Err(anyhow::anyhow!(
            "File too large for processing: {} bytes (exceeds quick reject threshold of {} bytes)",
            file_size,
            config.format_limits.min_quick_reject
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
                                    info!(
                                        "Detected PNG format for {}, applying {}MB limit",
                                        image_path,
                                        config.format_limits.png_max / (1024 * 1024)
                                    );
                                    config.format_limits.png_max
                                }
                                image::ImageFormat::Jpeg => {
                                    info!(
                                        "Detected JPEG format for {}, applying {}MB limit",
                                        image_path,
                                        config.format_limits.jpeg_max / (1024 * 1024)
                                    );
                                    config.format_limits.jpeg_max
                                }
                                image::ImageFormat::Bmp => {
                                    info!(
                                        "Detected BMP format for {}, applying {}MB limit",
                                        image_path,
                                        config.format_limits.bmp_max / (1024 * 1024)
                                    );
                                    config.format_limits.bmp_max
                                }
                                image::ImageFormat::Tiff => {
                                    info!(
                                        "Detected TIFF format for {}, applying {}MB limit",
                                        image_path,
                                        config.format_limits.tiff_max / (1024 * 1024)
                                    );
                                    config.format_limits.tiff_max
                                }
                                _ => {
                                    info!("Detected unsupported format {format:?} for {image_path}, using general limit");
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
                            info!(
                                "Estimated memory usage for {image_path}: {estimated_memory_mb}MB"
                            );

                            // Check if estimated memory usage exceeds safe limits
                            let max_memory_mb = 100.0; // 100MB memory limit for OCR processing
                            if estimated_memory_mb > max_memory_mb {
                                return Err(anyhow::anyhow!(
                                    "Estimated memory usage too high: {}MB (maximum allowed: {}MB). File would cause out-of-memory errors.",
                                    estimated_memory_mb, max_memory_mb
                                ));
                            }

                            Ok(())
                        }
                        Err(_) => {
                            // Could not determine format, use general limit
                            info!("Could not determine image format for {image_path}, using general size limit");
                            if file_size > config.max_file_size {
                                return Err(anyhow::anyhow!(
                                    "Image file too large: {} bytes (maximum allowed: {} bytes)",
                                    file_size,
                                    config.max_file_size
                                ));
                            }
                            Ok(())
                        }
                    }
                }
                _ => {
                    // Could not read enough bytes, use general limit
                    info!("Could not read enough bytes for format detection from {image_path}, using general size limit");
                    if file_size > config.max_file_size {
                        return Err(anyhow::anyhow!(
                            "Image file too large: {} bytes (maximum allowed: {} bytes)",
                            file_size,
                            config.max_file_size
                        ));
                    }
                    Ok(())
                }
            }
        }
        Err(e) => Err(anyhow::anyhow!(
            "Cannot open image file for validation: {} - {}",
            image_path,
            e
        )),
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
pub fn estimate_memory_usage(file_size: u64, format: &image::ImageFormat) -> f64 {
    // Convert file size to MB. Precision loss is acceptable for image files
    // as they rarely exceed sizes where f64 precision becomes an issue.
    #[allow(clippy::cast_precision_loss)]
    let file_size_mb = file_size as f64 / (1024.0 * 1024.0);

    // Memory estimation factors based on format characteristics
    let memory_factor = match format {
        image::ImageFormat::Png => 3.0, // PNG decompression can use 2-4x file size
        image::ImageFormat::Jpeg => 2.5, // JPEG decompression uses ~2-3x
        image::ImageFormat::Bmp => 1.2, // BMP is mostly uncompressed
        image::ImageFormat::Tiff => 4.0, // TIFF can be complex with layers
        _ => 3.0,                       // Default estimation
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
/// * `circuit_breaker` - Circuit breaker for fault tolerance and cascading failure prevention
///
/// # Returns
///
/// Returns `Result<String, OcrError>` containing the extracted text or an error
///
/// # Examples
///
/// ```rust,no_run
/// use ingredients::ocr::{extract_text_from_image, OcrConfig, OcrInstanceManager, CircuitBreaker};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = OcrConfig::default();
/// let instance_manager = OcrInstanceManager::new();
/// let circuit_breaker = CircuitBreaker::new(config.recovery.clone());
///
/// // Process an image of ingredients
/// let text = extract_text_from_image("/path/to/ingredients.jpg", &config, &instance_manager, &circuit_breaker).await?;
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
/// # Circuit Breaker Protection
///
/// The circuit breaker prevents cascading failures by:
/// - Opening when failure threshold is exceeded (default: 5 failures)
/// - Failing fast when open to protect system resources
/// - Automatically resetting after timeout (default: 60 seconds)
/// - Recording success/failure to track system health
///
/// # Errors
///
/// Returns `OcrError` for various failure conditions:
/// - `ValidationError` - Image format not supported or file too large
/// - `InitializationError` - OCR engine initialization failed
/// - `ImageLoadError` - Could not load the image file
/// - `ExtractionError` - OCR processing failed
/// - `TimeoutError` - Operation exceeded timeout (30s default)
pub async fn extract_text_from_image(
    image_path: &str,
    config: &crate::ocr_config::OcrConfig,
    instance_manager: &crate::instance_manager::OcrInstanceManager,
    circuit_breaker: &crate::circuit_breaker::CircuitBreaker,
) -> Result<String, crate::ocr_errors::OcrError> {
    // Start timing the entire OCR operation
    let start_time = std::time::Instant::now();

    // Check circuit breaker before processing
    if circuit_breaker.is_open() {
        warn!("Circuit breaker is open, rejecting OCR request for image: {image_path}");
        return Err(crate::ocr_errors::OcrError::Extraction(
            "OCR service is temporarily unavailable due to repeated failures. Please try again later.".to_string()
        ));
    }

    // Validate input with enhanced format-specific validation
    validate_image_with_format_limits(image_path, config)
        .map_err(|e| crate::ocr_errors::OcrError::Validation(e.to_string()))?;

    info!("Starting OCR text extraction from image: {image_path}");

    // Implement retry logic with exponential backoff
    let mut attempt = 0;
    let max_attempts = config.recovery.max_retries + 1; // +1 for initial attempt

    loop {
        attempt += 1;

        match perform_ocr_extraction(image_path, config, instance_manager).await {
            Ok(text) => {
                let total_duration = start_time.elapsed();
                let total_ms = total_duration.as_millis();

                // Record success in circuit breaker
                circuit_breaker.record_success();

                info!("OCR extraction completed successfully on attempt {} in {}ms. Extracted {} characters of text",
                      attempt, total_ms, text.len());
                return Ok(text);
            }
            Err(err) => {
                if attempt >= max_attempts {
                    let total_duration = start_time.elapsed();
                    let total_ms = total_duration.as_millis();

                    // Record failure in circuit breaker
                    circuit_breaker.record_failure();

                    error!("OCR extraction failed after {max_attempts} attempts ({total_ms}ms total): {err:?}");
                    return Err(err);
                }

                let delay_ms = calculate_retry_delay(attempt, &config.recovery);
                warn!("OCR extraction attempt {attempt} failed: {err:?}. Retrying in {delay_ms}ms");

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
async fn perform_ocr_extraction(
    image_path: &str,
    config: &crate::ocr_config::OcrConfig,
    instance_manager: &crate::instance_manager::OcrInstanceManager,
) -> Result<String, crate::ocr_errors::OcrError> {
    // Start timing the actual OCR processing
    let ocr_start_time = std::time::Instant::now();

    // Create a timeout for the operation
    let timeout_duration = tokio::time::Duration::from_secs(config.recovery.operation_timeout_secs);

    let result = tokio::time::timeout(timeout_duration, async {
        // Get or create OCR instance from the manager
        let instance = instance_manager
            .get_instance(config)
            .map_err(|e| crate::ocr_errors::OcrError::Initialization(e.to_string()))?;

        // Perform OCR processing with the reused instance
        let extracted_text = {
            let mut tess = instance.lock().unwrap();
            // Set the image for OCR processing
            tess.set_image(image_path).map_err(|e| {
                crate::ocr_errors::OcrError::ImageLoad(format!("Failed to load image for OCR: {e}"))
            })?;

            // Extract text from the image
            tess.get_utf8_text().map_err(|e| {
                crate::ocr_errors::OcrError::Extraction(format!(
                    "Failed to extract text from image: {e}"
                ))
            })?
        };

        // Clean up the extracted text (remove extra whitespace and empty lines)
        let cleaned_text = extracted_text
            .trim()
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect::<Vec<&str>>()
            .join("\n");

        Ok(cleaned_text)
    })
    .await;

    let ocr_duration = ocr_start_time.elapsed();
    let ocr_ms = ocr_duration.as_millis();

    match result {
        Ok(Ok(text)) => {
            info!(
                "OCR processing completed in {}ms, extracted {} characters",
                ocr_ms,
                text.len()
            );
            Ok(text)
        }
        Ok(Err(e)) => {
            warn!("OCR processing failed after {ocr_ms}ms: {e:?}");
            Err(e)
        }
        Err(_) => {
            warn!(
                "OCR processing timed out after {}ms (limit: {}s)",
                ocr_ms, config.recovery.operation_timeout_secs
            );
            Err(crate::ocr_errors::OcrError::Timeout(format!(
                "OCR operation timed out after {} seconds",
                config.recovery.operation_timeout_secs
            )))
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
pub fn calculate_retry_delay(attempt: u32, recovery: &crate::ocr_config::RecoveryConfig) -> u64 {
    // Calculate exponential backoff with minimal precision loss
    // For retry delays, precision loss is acceptable as delays are typically small
    #[allow(clippy::cast_precision_loss)]
    let base_delay = recovery.base_retry_delay_ms as f64;

    #[allow(clippy::cast_precision_loss)]
    let exponential_delay = base_delay * (2.0_f64).powf((attempt - 1) as f64);

    #[allow(clippy::cast_precision_loss)]
    let delay = exponential_delay.min(recovery.max_retry_delay_ms as f64) as u64;

    // Add some jitter to prevent thundering herd
    let jitter = (rand::random::<u64>() % (delay / 4)) as u64;
    delay + jitter
}

/// Validate if an image file is supported for OCR processing using `image::guess_format`
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
pub fn is_supported_image_format(file_path: &str, config: &crate::ocr_config::OcrConfig) -> bool {
    // Enhanced validation first (includes size checks)
    if validate_image_with_format_limits(file_path, config).is_err() {
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

                    info!("Read {bytes_read} bytes from file {file_path} for format detection");

                    match image::guess_format(&buffer) {
                        Ok(format) => {
                            // Tesseract supports: PNG, JPEG/JPG, BMP, TIFF
                            let supported = matches!(
                                format,
                                image::ImageFormat::Png
                                    | image::ImageFormat::Jpeg
                                    | image::ImageFormat::Bmp
                                    | image::ImageFormat::Tiff
                            );

                            if supported {
                                info!("Detected supported image format: {format:?} for file: {file_path}");
                            } else {
                                info!("Detected unsupported image format: {format:?} for file: {file_path}");
                            }

                            supported
                        }
                        Err(e) => {
                            info!("Could not determine image format for file: {file_path} - {e}");
                            false
                        }
                    }
                }
                Ok(bytes_read) => {
                    info!("Could not read enough bytes to determine image format for file: {} (read {} bytes, need at least {})", file_path, bytes_read, config.min_format_bytes);
                    false
                }
                Err(e) => {
                    info!("Error reading image file for format detection: {file_path} - {e}");
                    false
                }
            }
        }
        Err(e) => {
            info!("Could not open image file for format detection: {file_path} - {e}");
            false
        }
    }
}
