use rusqlite::{Connection, Result};
use std::env;

fn main() -> Result<()> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();

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

    println!("Database initialized successfully at {}", database_url);

    Ok(())
}
