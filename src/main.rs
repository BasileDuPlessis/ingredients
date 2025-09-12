use rusqlite::Connection;
use std::env;
use teloxide::prelude::*;
use anyhow::Result;
use log::info;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::Duration;

mod db;
mod bot;
mod ocr;

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

    info!("Initializing database at: {database_url}");

    // Create database connection
    let conn = Connection::open(&database_url)?;

    // Initialize database schema
    db::init_database_schema(&conn)?;

    // Wrap connection in Arc<Mutex> for sharing across async tasks
    let shared_conn = Arc::new(Mutex::new(conn));

    // Initialize the bot with custom client configuration for better reliability
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))  // 30 second timeout
        .build()
        .expect("Failed to create HTTP client");

    let bot = Bot::with_client(bot_token, client);

    info!("Bot initialized with 30s timeout, starting dispatcher");

    // Set up the dispatcher with shared connection
    let handler = dptree::entry()
        .branch(Update::filter_message().endpoint({
            let conn = Arc::clone(&shared_conn);
            move |bot: Bot, msg: Message| {
                let conn = Arc::clone(&conn);
                async move { bot::message_handler(bot, msg, conn).await }
            }
        }));

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}