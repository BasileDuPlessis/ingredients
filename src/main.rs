use anyhow::Result;
use ingredients::bot;
use ingredients::db;
use ingredients::localization;
use log::info;
use sqlx::postgres::PgPool;
use std::env;
use std::sync::Arc;
use std::time::Duration;
use teloxide::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();

    // Initialize localization
    localization::init_localization()?;

    info!("Starting Ingredients Telegram Bot");

    // Load environment variables from .env file
    dotenv::dotenv().ok();

    // Get bot token from environment
    let bot_token = env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN must be set");

    // Get database path from environment
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    info!("Initializing database at: {database_url}");

    // Create database connection pool
    let pool = PgPool::connect(&database_url).await?;

    // Initialize database schema
    db::init_database_schema(&pool).await?;

    // Wrap pool in Arc for sharing across async tasks
    let shared_pool = Arc::new(pool);

    // Initialize the bot with custom client configuration for better reliability
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30)) // 30 second timeout
        .build()
        .expect("Failed to create HTTP client");

    let bot = Bot::with_client(bot_token, client);

    info!("Bot initialized with 30s timeout, starting dispatcher");

    // Set up the dispatcher with shared connection
    let handler = dptree::entry().branch(Update::filter_message().endpoint({
        let pool = Arc::clone(&shared_pool);
        move |bot: Bot, msg: Message| {
            let pool = Arc::clone(&pool);
            async move { bot::message_handler(bot, msg, pool).await }
        }
    }));

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}
