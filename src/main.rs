use rusqlite::Connection;
use std::env;
use teloxide::prelude::*;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();

    // Get bot token from environment
    let bot_token = env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN must be set");

    // Get database path from environment
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // Create database connection
    let conn = Connection::open(&database_url)?;

    // Create entries table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS entries (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            telegram_id INTEGER NOT NULL,
            content TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    // Create FTS virtual table for full-text search
    conn.execute(
        "CREATE VIRTUAL TABLE IF NOT EXISTS entries_fts USING fts5(
            content,
            content='entries',
            content_rowid='id'
        )",
        [],
    )?;

    // Create triggers to keep FTS table in sync
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS entries_insert AFTER INSERT ON entries
         BEGIN
             INSERT INTO entries_fts(rowid, content) VALUES (new.id, new.content);
         END",
        [],
    )?;

    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS entries_delete AFTER DELETE ON entries
         BEGIN
             DELETE FROM entries_fts WHERE rowid = old.id;
         END",
        [],
    )?;

    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS entries_update AFTER UPDATE ON entries
         BEGIN
             UPDATE entries_fts SET content = new.content WHERE rowid = new.id;
         END",
        [],
    )?;

    println!("Database initialized successfully");

    // Initialize the bot
    let bot = Bot::new(bot_token);

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
        bot.send_message(msg.chat.id, format!("Received: {}", text)).await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    #[test]
    fn test_database_initialization() {
        // Create an in-memory database for testing
        let conn = Connection::open_in_memory().unwrap();

        // Test creating the entries table
        let result = conn.execute(
            "CREATE TABLE IF NOT EXISTS entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                telegram_id INTEGER NOT NULL,
                content TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        );

        assert!(result.is_ok());

        // Test creating the FTS table
        let result = conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS entries_fts USING fts5(
                content,
                content='entries',
                content_rowid='id'
            )",
            [],
        );

        assert!(result.is_ok());

        // Test inserting a record
        let result = conn.execute(
            "INSERT INTO entries (telegram_id, content) VALUES (?1, ?2)",
            rusqlite::params![12345, "Test content"],
        );

        assert!(result.is_ok());

        // Test querying the record
        let mut stmt = conn.prepare("SELECT id, telegram_id, content FROM entries WHERE id = 1").unwrap();
        let mut rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
            ))
        }).unwrap();

        if let Some(row) = rows.next() {
            let (id, telegram_id, content) = row.unwrap();
            assert_eq!(id, 1);
            assert_eq!(telegram_id, 12345);
            assert_eq!(content, "Test content");
        } else {
            panic!("No row found");
        }
    }
}
