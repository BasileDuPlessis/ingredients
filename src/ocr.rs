use leptess::LepTess;
use std::fs::File;
use std::io::{BufReader, Read};
use anyhow::Result;
use log::info;

/// Extract text from an image using Tesseract OCR
pub async fn extract_text_from_image(image_path: &str) -> Result<String> {
    info!("Starting OCR text extraction from image: {}", image_path);

    // Check if the file exists and is readable
    if !std::path::Path::new(image_path).exists() {
        return Err(anyhow::anyhow!("Image file does not exist: {}", image_path));
    }

    // Create a new Tesseract instance with English and French languages
    let mut tess = LepTess::new(None, "eng+fra")
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
    match File::open(file_path) {
        Ok(file) => {
            let mut reader = BufReader::new(file);
            let mut buffer = vec![0; 32]; // Pre-allocate 32 bytes

            match reader.read(&mut buffer) {
                Ok(bytes_read) if bytes_read >= 8 => {
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
                    info!("Could not read enough bytes to determine image format for file: {} (read {} bytes, need at least 8)", file_path, bytes_read);
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
