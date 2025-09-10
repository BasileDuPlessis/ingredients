use teloxide::prelude::*;
use teloxide::types::FileId;
use tempfile::NamedTempFile;
use std::io::Write;
use std::sync::Arc;
use tokio::sync::Mutex;
use rusqlite::Connection;
use anyhow::Result;
use log::{info, error};

// Create OCR configuration with default settings
lazy_static::lazy_static! {
    static ref OCR_CONFIG: crate::ocr::OcrConfig = crate::ocr::OcrConfig::default();
    static ref OCR_INSTANCE_MANAGER: crate::ocr::OcrInstanceManager = crate::ocr::OcrInstanceManager::default();
    static ref CIRCUIT_BREAKER: crate::ocr::CircuitBreaker = crate::ocr::CircuitBreaker::new(OCR_CONFIG.recovery.clone());
}

async fn download_file(bot: &Bot, file_id: FileId) -> Result<String> {
    let file = bot.get_file(file_id).await?;
    let file_path = file.path;
    let url = format!("https://api.telegram.org/file/bot{}/{}", bot.token(), file_path);

    let response = reqwest::get(&url).await?;
    let bytes = response.bytes().await?;

    let mut temp_file = NamedTempFile::new()?;
    temp_file.as_file_mut().write_all(&bytes)?;
    let path = temp_file.path().to_string_lossy().to_string();
    temp_file.keep()?; // Keep the file from being deleted

    Ok(path)
}

async fn download_and_process_image(
    bot: &Bot,
    file_id: FileId,
    chat_id: ChatId,
    success_message: &str,
) -> Result<String> {
    match download_file(bot, file_id).await {
        Ok(temp_path) => {
            info!("Image downloaded to: {}", temp_path);

            // Send initial success message
            bot.send_message(chat_id, success_message).await?;

            // Validate image format before OCR processing
            if !crate::ocr::is_supported_image_format(&temp_path, &OCR_CONFIG) {
                info!("Unsupported image format for user {}", chat_id);
                bot.send_message(chat_id, "❌ Unsupported image format. Please use PNG, JPG, JPEG, BMP, TIFF, or TIF formats.").await?;
                return Ok(String::new());
            }

            // Extract text from the image using OCR with circuit breaker protection
            match crate::ocr::extract_text_from_image(&temp_path, &OCR_CONFIG, &OCR_INSTANCE_MANAGER, &CIRCUIT_BREAKER).await {
                Ok(extracted_text) => {
                    if extracted_text.is_empty() {
                        info!("No text found in image from user {}", chat_id);
                        bot.send_message(chat_id, "⚠️ No text was found in the image. Please try a clearer image with visible text.").await?;
                        Ok(String::new())
                    } else {
                        info!("Successfully extracted {} characters of text from user {}", extracted_text.len(), chat_id);

                        // Send the extracted text back to the user
                        let response_message = format!(
                            "✅ **Text extracted successfully!**\n\n📝 **Extracted Text:**\n```\n{}\n```",
                            extracted_text
                        );
                        bot.send_message(chat_id, &response_message).await?;

                        Ok(extracted_text)
                    }
                }
                Err(e) => {
                    error!("OCR processing failed for user {}: {:?}", chat_id, e);

                    // Provide more specific error messages based on the error type
                    let error_message = match &e {
                        crate::ocr::OcrError::ValidationError(msg) => {
                            format!("❌ Image validation failed: {}", msg)
                        }
                        crate::ocr::OcrError::ImageLoadError(_) => {
                            "❌ The image format is not supported or the image is corrupted. Please try with a PNG, JPG, or BMP image.".to_string()
                        }
                        crate::ocr::OcrError::InitializationError(_) => {
                            "❌ OCR engine initialization failed. Please try again later.".to_string()
                        }
                        crate::ocr::OcrError::ExtractionError(_) => {
                            "❌ Failed to extract text from the image. Please try again with a different image.".to_string()
                        }
                        crate::ocr::OcrError::TimeoutError(msg) => {
                            format!("❌ OCR processing timed out: {}", msg)
                        }
                        crate::ocr::OcrError::_InstanceCorruptionError(_) => {
                            "❌ OCR engine encountered an internal error. Please try again.".to_string()
                        }
                        crate::ocr::OcrError::_ResourceExhaustionError(_) => {
                            "❌ System resources are exhausted. Please try again later.".to_string()
                        }
                    };

                    bot.send_message(chat_id, &error_message).await?;
                    Err(anyhow::anyhow!("OCR processing failed: {:?}", e))
                }
            }
        }
        Err(e) => {
            error!("Failed to download image for user {}: {:?}", chat_id, e);
            bot.send_message(chat_id, "❌ Failed to download the image. Please try again.").await?;
            Err(e)
        }
    }
}

async fn handle_text_message(bot: &Bot, msg: &Message) -> Result<()> {
    if let Some(text) = msg.text() {
        info!("Received text message from user {}: {}", msg.chat.id, text);
        bot.send_message(msg.chat.id, format!("Received: {}", text)).await?;
    }
    Ok(())
}

async fn handle_photo_message(bot: &Bot, msg: &Message) -> Result<()> {
    info!("Received photo from user {}", msg.chat.id);
    if let Some(photos) = msg.photo() {
        if let Some(largest_photo) = photos.last() {
            let _temp_path = download_and_process_image(
                bot,
                largest_photo.file.id.clone(),
                msg.chat.id,
                "Photo downloaded successfully! Processing...",
            ).await;
        }
    }
    Ok(())
}

async fn handle_document_message(bot: &Bot, msg: &Message) -> Result<()> {
    if let Some(doc) = msg.document() {
        if let Some(mime_type) = &doc.mime_type {
            if mime_type.to_string().starts_with("image/") {
                info!("Received image document from user {}", msg.chat.id);
                let _temp_path = download_and_process_image(
                    bot,
                    doc.file.id.clone(),
                    msg.chat.id,
                    "Image document downloaded successfully! Processing...",
                ).await;
            } else {
                info!("Received non-image document from user {}", msg.chat.id);
                bot.send_message(msg.chat.id, "Received a document, but it's not an image.").await?;
            }
        } else {
            info!("Received document without MIME type from user {}", msg.chat.id);
            bot.send_message(msg.chat.id, "Received a document. Unable to determine if it's an image.").await?;
        }
    }
    Ok(())
}

async fn handle_unsupported_message(bot: &Bot, msg: &Message) -> Result<()> {
    info!("Received unsupported message type from user {}", msg.chat.id);
    bot.send_message(msg.chat.id, "Sorry, I can only process text or images.").await?;
    Ok(())
}

pub async fn message_handler(
    bot: Bot,
    msg: Message,
    _conn: Arc<Mutex<Connection>>, // TODO: Use for database operations when OCR is implemented
) -> Result<()> {
    if msg.text().is_some() {
        handle_text_message(&bot, &msg).await?;
    } else if msg.photo().is_some() {
        handle_photo_message(&bot, &msg).await?;
    } else if msg.document().is_some() {
        handle_document_message(&bot, &msg).await?;
    } else {
        handle_unsupported_message(&bot, &msg).await?;
    }

    Ok(())
}
