use anyhow::Result;
use ingredients::bot;
use ingredients::db;
use ingredients::dialogue::{RecipeDialogue, RecipeDialogueState};
use ingredients::localization;
use sqlx::postgres::PgPool;
use std::env;
use std::sync::Arc;
use std::time::Duration;
use teloxide::dispatching::dialogue::InMemStorage;
use teloxide::prelude::*;
use tracing::{info, Level};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize structured logging with module-specific filtering
    init_tracing();

    // Initialize localization
    localization::init_localization()?;

    info!("Starting Ingredients Telegram Bot");

    // Load environment variables from .env file
    dotenv::dotenv().ok();

    // Get bot token from environment
    let bot_token = env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN must be set");

    // Get database path from environment
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    info!(database_url = %database_url, "Initializing database connection");

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

        // Create shared dialogue storage
    let dialogue_storage = InMemStorage::<RecipeDialogueState>::new();

    // Set up the dispatcher with shared connection and dialogue support
    let handler = dptree::entry()
        .branch(Update::filter_message().endpoint({
            let pool = Arc::clone(&shared_pool);
            let storage = dialogue_storage.clone();
            move |bot: Bot, msg: Message| {
                let pool = Arc::clone(&pool);
                let storage = storage.clone();
                let dialogue = RecipeDialogue::new(storage, msg.chat.id);
                async move { bot::message_handler(bot, msg, pool, dialogue).await }
            }
        }))
        .branch(Update::filter_callback_query().endpoint({
            let pool = Arc::clone(&shared_pool);
            let storage = dialogue_storage.clone();
            move |bot: Bot, q: CallbackQuery| {
                let pool = Arc::clone(&pool);
                let storage = storage.clone();
                // Use the chat ID from the original message that contained the inline keyboard
                let chat_id = match &q.message {
                    Some(msg) => match msg {
                        teloxide::types::MaybeInaccessibleMessage::Regular(msg) => msg.chat.id,
                        teloxide::types::MaybeInaccessibleMessage::Inaccessible(_) => {
                            ChatId::from(q.from.id)
                        }
                    },
                    None => ChatId::from(q.from.id),
                };
                let dialogue = RecipeDialogue::new(storage, chat_id);
                async move { bot::callback_handler(bot, q, pool, dialogue).await }
            }
        }));

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}

fn init_tracing() {
    // Create a filter that allows INFO level by default, but DEBUG for specific modules
    let filter = EnvFilter::builder()
        .with_default_directive(Level::INFO.into())
        // Allow environment variable override
        .with_env_var("RUST_LOG")
        .from_env_lossy();

    // Initialize tracing with JSON formatting for production readiness
    let log_format = env::var("LOG_FORMAT").unwrap_or_else(|_| "pretty".to_string());

    if log_format == "json" {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().json())
            .init();
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().pretty())
            .init();
    }
}
