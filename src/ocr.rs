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
use anyhow::Result;
use log::info;

// Constants for OCR configuration
const DEFAULT_LANGUAGES: &str = "eng+fra";
const FORMAT_DETECTION_BUFFER_SIZE: usize = 32;
const MIN_FORMAT_BYTES: usize = 8;
const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10MB limit for image files

/// Validate image file path and basic properties
fn validate_image_path(image_path: &str) -> Result<()> {
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
            if file_size > MAX_FILE_SIZE {
                return Err(anyhow::anyhow!(
                    "Image file too large: {} bytes (maximum allowed: {} bytes)",
                    file_size,
                    MAX_FILE_SIZE
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

/// Extract text from an image using Tesseract OCR
pub async fn extract_text_from_image(image_path: &str) -> Result<String> {
    // Validate input before processing
    validate_image_path(image_path)?;

    info!("Starting OCR text extraction from image: {}", image_path);

    // Create a new Tesseract instance with English and French languages
    let mut tess = LepTess::new(None, DEFAULT_LANGUAGES)
        .map_err(|e| anyhow::anyhow!("Failed to initialize Tesseract OCR: {}", e))?;

    // Set the image for OCR processing
    tess.set_image(image_path)
        .map_err(|e| anyhow::anyhow!("Failed to load image for OCR: {}", e))?;

    // Extract text from the image
    let extracted_text = tess.get_utf8_text()
        .map_err(|e| anyhow::anyhow!("Failed to extract text from image: {}", e))?;

    // Clean up the extracted text (remove extra whitespace and empty lines)
    let cleaned_text = extracted_text
        .trim()
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<&str>>()
        .join("\n");

    info!("OCR extraction completed. Extracted {} characters of text", cleaned_text.len());

    Ok(cleaned_text)
}

/// Validate if an image file is supported for OCR processing using image::guess_format
pub fn is_supported_image_format(file_path: &str) -> bool {
    // Basic validation first
    if let Err(_) = validate_image_path(file_path) {
        return false;
    }

    match File::open(file_path) {
        Ok(file) => {
            let mut reader = BufReader::new(file);
            let mut buffer = vec![0; FORMAT_DETECTION_BUFFER_SIZE]; // Pre-allocate buffer for format detection

            match reader.read(&mut buffer) {
                Ok(bytes_read) if bytes_read >= MIN_FORMAT_BYTES => {
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
                    info!("Could not read enough bytes to determine image format for file: {} (read {} bytes, need at least {})", file_path, bytes_read, MIN_FORMAT_BYTES);
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
