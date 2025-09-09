use rusqlite::Connection;
use std::env;
use teloxide::prelude::*;
use anyhow::Result;
use log::info;
use teloxide::types::FileId;
use tempfile::NamedTempFile;
use std::io::Write;
use std::sync::Arc;
use tokio::sync::Mutex;

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

async fn download_and_process_image(
    bot: &Bot,
    file_id: FileId,
    chat_id: ChatId,
    success_message: &str,
) -> Result<String> {
    match download_file(bot, file_id).await {
        Ok(temp_path) => {
            info!("Image downloaded to: {}", temp_path);
            bot.send_message(chat_id, success_message).await?;
            // TODO: Process the image with OCR
            Ok(temp_path)
        }
        Err(e) => {
            info!("Failed to download image: {:?}", e);
            bot.send_message(chat_id, "Failed to download image.").await?;
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