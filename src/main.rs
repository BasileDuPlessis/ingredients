use rusqlite::Connection;
use std::env;
use teloxide::prelude::*;
use anyhow::Result;
use log::info;

mod db;

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

    // Initialize the bot
    let bot = Bot::new(bot_token);

    info!("Bot initialized, starting dispatcher");

    // Set up the dispatcher
    let handler = dptree::entry()
        .branch(Update::filter_message().endpoint(message_handler));

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
) -> Result<()> {
    if let Some(text) = msg.text() {
        info!("Received text message from user {}: {}", msg.chat.id, text);
        bot.send_message(msg.chat.id, format!("Received: {}", text)).await?;
    } else {
        info!("Received non-text message from user {}", msg.chat.id);
    }

    Ok(())
}
