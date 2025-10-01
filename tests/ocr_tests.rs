//! # OCR Tests Module
//!
//! Comprehensive test suite for OCR processing functionality,
//! including configuration, validation, circuit breaker, and instance management.

#[cfg(test)]
mod tests {
    use ingredients::circuit_breaker::CircuitBreaker;
    use ingredients::instance_manager::OcrInstanceManager;
    use ingredients::ocr::{
        calculate_retry_delay, estimate_memory_usage, is_supported_image_format,
        validate_image_path, validate_image_with_format_limits,
    };
    use ingredients::ocr_config::{FormatSizeLimits, OcrConfig, RecoveryConfig};
    use ingredients::ocr_errors::OcrError;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Test OCR configuration defaults
    #[test]
    fn test_ocr_config_defaults() {
        let config = OcrConfig::default();

        assert_eq!(config.languages, "eng+fra");
        assert_eq!(config.buffer_size, 32);
        assert_eq!(config.min_format_bytes, 8);
        assert_eq!(config.max_file_size, 10 * 1024 * 1024);
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

        assert_eq!(limits.png_max, 15 * 1024 * 1024); // 15MB
        assert_eq!(limits.jpeg_max, 10 * 1024 * 1024); // 10MB
        assert_eq!(limits.bmp_max, 5 * 1024 * 1024); // 5MB
        assert_eq!(limits.tiff_max, 20 * 1024 * 1024); // 20MB
        assert_eq!(limits.min_quick_reject, 50 * 1024 * 1024); // 50MB
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
        assert!(std::sync::Arc::ptr_eq(&instance1, &instance2));

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
            OcrError::Extraction(msg) => assert!(msg.contains("test error")),
            _ => panic!("Expected Extraction"),
        }

        // Test Display implementation
        let error = OcrError::Validation("test".to_string());
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
        let config = OcrConfig {
            max_file_size: 100, // Very small limit
            ..Default::default()
        };

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

    /// Test circuit breaker integration with extract_text_from_image
    #[test]
    fn test_extract_text_from_image_circuit_breaker_integration() {
        let config = OcrConfig::default();
        let instance_manager = OcrInstanceManager::new();
        let circuit_breaker = CircuitBreaker::new(config.recovery.clone());

        // Initially circuit breaker should be closed
        assert!(!circuit_breaker.is_open());

        // Create a temporary file for testing
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"test content").unwrap();
        let temp_path = temp_file.path().to_string_lossy().to_string();

        // Test that function can be called with circuit breaker parameter
        // This verifies the function signature accepts the circuit breaker
        let _future = ingredients::ocr::extract_text_from_image(
            &temp_path,
            &config,
            &instance_manager,
            &circuit_breaker,
        );
        // The function compiles and can be called with 4 parameters as expected
    }

    #[test]
    fn test_validate_image_format_valid_png() {
        let config = OcrConfig::default();

        // Create mock PNG file
        let mut temp_file = NamedTempFile::new().unwrap();
        let png_header = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        temp_file.write_all(&png_header).unwrap();
        temp_file.write_all(&vec![0u8; 1000]).unwrap(); // 1KB content
        let temp_path = temp_file.path().to_string_lossy().to_string();

        let result = validate_image_with_format_limits(&temp_path, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_image_format_valid_jpeg() {
        let config = OcrConfig::default();

        // Create mock JPEG file
        let mut temp_file = NamedTempFile::new().unwrap();
        let jpeg_header = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46];
        temp_file.write_all(&jpeg_header).unwrap();
        temp_file.write_all(&vec![0u8; 2000000]).unwrap(); // 2MB content (under JPEG limit)
        let temp_path = temp_file.path().to_string_lossy().to_string();

        let result = validate_image_with_format_limits(&temp_path, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_image_format_unsupported_format() {
        let config = OcrConfig::default();

        // Create file with unsupported format
        let mut temp_file = NamedTempFile::new().unwrap();
        let unsupported_header = [0x00, 0x00, 0x00, 0x00];
        temp_file.write_all(&unsupported_header).unwrap();
        temp_file.write_all(&vec![0u8; 1000]).unwrap();
        let temp_path = temp_file.path().to_string_lossy().to_string();

        // Test that is_supported_image_format returns false for unsupported formats
        let is_supported = is_supported_image_format(&temp_path, &config);
        assert!(!is_supported, "Unsupported format should not be supported");

        // But validate_image_with_format_limits should still pass (uses general limit)
        let result = validate_image_with_format_limits(&temp_path, &config);
        assert!(
            result.is_ok(),
            "Validation should pass for unsupported format (uses general limit)"
        );
    }

    #[test]
    fn test_validate_image_format_png_too_large() {
        let config = OcrConfig::default();

        // Create PNG file that's too large
        let mut temp_file = NamedTempFile::new().unwrap();
        let png_header = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        temp_file.write_all(&png_header).unwrap();
        temp_file.write_all(&vec![0u8; 20 * 1024 * 1024]).unwrap(); // 20MB (over PNG limit)
        let temp_path = temp_file.path().to_string_lossy().to_string();

        let result = validate_image_with_format_limits(&temp_path, &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_image_format_jpeg_too_large() {
        let config = OcrConfig::default();

        // Create JPEG file that's too large
        let mut temp_file = NamedTempFile::new().unwrap();
        let jpeg_header = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46];
        temp_file.write_all(&jpeg_header).unwrap();
        temp_file.write_all(&vec![0u8; 12 * 1024 * 1024]).unwrap(); // 12MB (over JPEG limit)
        let temp_path = temp_file.path().to_string_lossy().to_string();

        let result = validate_image_with_format_limits(&temp_path, &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_estimate_memory_usage_different_sizes() {
        // Test reasonable memory estimation for different file sizes and formats
        let file_size_1mb = 1024 * 1024;

        // Test PNG format (highest memory factor)
        let png_memory = estimate_memory_usage(file_size_1mb, &image::ImageFormat::Png);
        assert_eq!(png_memory, 3.0); // 1MB * 3.0 = 3MB

        // Test JPEG format
        let jpeg_memory = estimate_memory_usage(file_size_1mb, &image::ImageFormat::Jpeg);
        assert_eq!(jpeg_memory, 2.5); // 1MB * 2.5 = 2.5MB

        // Test BMP format (lowest memory factor)
        let bmp_memory = estimate_memory_usage(file_size_1mb, &image::ImageFormat::Bmp);
        assert_eq!(bmp_memory, 1.2); // 1MB * 1.2 = 1.2MB

        // Test TIFF format
        let tiff_memory = estimate_memory_usage(file_size_1mb, &image::ImageFormat::Tiff);
        assert_eq!(tiff_memory, 4.0); // 1MB * 4.0 = 4MB

        // Test larger file
        let file_size_5mb = 5 * 1024 * 1024;
        let large_png_memory = estimate_memory_usage(file_size_5mb, &image::ImageFormat::Png);
        assert_eq!(large_png_memory, 15.0); // 5MB * 3.0 = 15MB

        // Test unknown format (should use default factor of 3.0)
        let unknown_memory = estimate_memory_usage(file_size_1mb, &image::ImageFormat::WebP);
        assert_eq!(unknown_memory, 3.0); // 1MB * 3.0 = 3MB (default)
    }
}
