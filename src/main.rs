use rusqlite::Connection;
use std::env;
use teloxide::prelude::*;
use anyhow::Result;
use log::{info, error};
use teloxide::types::FileId;
use tempfile::NamedTempFile;
use std::io::Write;
use std::sync::Arc;
use tokio::sync::Mutex;
use leptess::LepTess;
use std::fs::File;
use std::io::{BufReader, Read};

mod db;

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

/// Extract text from an image using Tesseract OCR
async fn extract_text_from_image(image_path: &str) -> Result<String> {
    info!("Starting OCR text extraction from image: {}", image_path);

    // Check if the file exists and is readable
    if !std::path::Path::new(image_path).exists() {
        return Err(anyhow::anyhow!("Image file does not exist: {}", image_path));
    }

    // Create a new Tesseract instance with English language
    let mut tess = LepTess::new(None, "eng")
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
fn is_supported_image_format(file_path: &str) -> bool {
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
            if !is_supported_image_format(&temp_path) {
                info!("Unsupported image format for user {}", chat_id);
                bot.send_message(chat_id, "❌ Unsupported image format. Please use PNG, JPG, JPEG, BMP, TIFF, or TIF formats.").await?;
                return Ok(String::new());
            }

            // Extract text from the image using OCR
            match extract_text_from_image(&temp_path).await {
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
                    let error_message = if e.to_string().contains("does not exist") {
                        "❌ Image file could not be processed. Please try uploading the image again."
                    } else if e.to_string().contains("Failed to load image") {
                        "❌ The image format is not supported or the image is corrupted. Please try with a PNG, JPG, or BMP image."
                    } else if e.to_string().contains("Failed to initialize Tesseract") {
                        "❌ OCR engine initialization failed. Please try again later."
                    } else {
                        "❌ Failed to extract text from the image. Please try again with a different image."
                    };

                    bot.send_message(chat_id, error_message).await?;
                    Err(e)
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

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();

    info!("Starting Ingredients Telegram Bot");

    // Load environment variables from .env file
    dotenv::dotenv().ok();

    // Get bot token from environment
    let bot_token = env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN must be set");

    // Get database path from environment
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    info!("Initializing database at: {}", database_url);

    // Create database connection
    let conn = Connection::open(&database_url)?;

    // Initialize database schema
    db::init_database_schema(&conn)?;

    // Wrap connection in Arc<Mutex> for sharing across async tasks
    let shared_conn = Arc::new(Mutex::new(conn));

    // Initialize the bot
    let bot = Bot::new(bot_token);

    info!("Bot initialized, starting dispatcher");

    // Set up the dispatcher with shared connection
    let handler = dptree::entry()
        .branch(Update::filter_message().endpoint({
            let conn = Arc::clone(&shared_conn);
            move |bot: Bot, msg: Message| {
                let conn = Arc::clone(&conn);
                async move { message_handler(bot, msg, conn).await }
            }
        }));

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}

async fn message_handler(
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