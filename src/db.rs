use anyhow::{Context, Result};
use log::info;
use rusqlite::{params, Connection};

/// Represents an entry in the database
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub id: i64,
    pub telegram_id: i64,
    pub content: String,
    pub created_at: String,
}

/// Initialize the database schema
pub fn init_database_schema(conn: &Connection) -> Result<()> {
    info!("Initializing database schema...");

    // Create entries table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS entries (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            telegram_id INTEGER NOT NULL,
            content TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )
    .context("Failed to create entries table")?;

    // Create FTS virtual table for full-text search
    conn.execute(
        "CREATE VIRTUAL TABLE IF NOT EXISTS entries_fts USING fts5(
            content,
            content='entries',
            content_rowid='id'
        )",
        [],
    )
    .context("Failed to create FTS table")?;

    // Create triggers to keep FTS table in sync
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS entries_insert AFTER INSERT ON entries
         BEGIN
             INSERT INTO entries_fts(rowid, content) VALUES (new.id, new.content);
         END",
        [],
    )
    .context("Failed to create insert trigger")?;

    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS entries_delete AFTER DELETE ON entries
         BEGIN
             DELETE FROM entries_fts WHERE rowid = old.id;
         END",
        [],
    )
    .context("Failed to create delete trigger")?;

    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS entries_update AFTER UPDATE ON entries
         BEGIN
             UPDATE entries_fts SET content = new.content WHERE rowid = new.id;
         END",
        [],
    )
    .context("Failed to create update trigger")?;

    info!("Database schema initialized successfully");
    Ok(())
}

/// Create a new entry in the database
pub fn create_entry(conn: &Connection, telegram_id: i64, content: &str) -> Result<i64> {
    info!("Creating new entry for telegram_id: {}", telegram_id);

    conn.execute(
        "INSERT INTO entries (telegram_id, content) VALUES (?1, ?2)",
        params![telegram_id, content],
    )
    .context("Failed to insert new entry")?;

    let entry_id = conn.last_insert_rowid();
    info!("Entry created with ID: {}", entry_id);

    Ok(entry_id)
}

/// Read an entry from the database by ID
pub fn read_entry(conn: &Connection, entry_id: i64) -> Result<Option<Entry>> {
    info!("Reading entry with ID: {}", entry_id);

    let mut stmt = conn
        .prepare("SELECT id, telegram_id, content, created_at FROM entries WHERE id = ?1")
        .context("Failed to prepare read statement")?;

    let entry = stmt.query_row(params![entry_id], |row| {
        Ok(Entry {
            id: row.get(0)?,
            telegram_id: row.get(1)?,
            content: row.get(2)?,
            created_at: row.get(3)?,
        })
    });

    match entry {
        Ok(entry) => {
            info!("Entry found with ID: {}", entry_id);
            Ok(Some(entry))
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            info!("No entry found with ID: {}", entry_id);
            Ok(None)
        }
        Err(e) => Err(e).context("Failed to read entry"),
    }
}

/// Update an existing entry in the database
pub fn update_entry(conn: &Connection, entry_id: i64, new_content: &str) -> Result<bool> {
    info!("Updating entry with ID: {}", entry_id);

    let rows_affected = conn
        .execute(
            "UPDATE entries SET content = ?1 WHERE id = ?2",
            params![new_content, entry_id],
        )
        .context("Failed to update entry")?;

    if rows_affected > 0 {
        info!("Entry updated successfully with ID: {}", entry_id);
        Ok(true)
    } else {
        info!("No entry found with ID: {}", entry_id);
        Ok(false)
    }
}

/// Delete an entry from the database
pub fn delete_entry(conn: &Connection, entry_id: i64) -> Result<bool> {
    info!("Deleting entry with ID: {}", entry_id);

    let rows_affected = conn
        .execute("DELETE FROM entries WHERE id = ?1", params![entry_id])
        .context("Failed to delete entry")?;

    if rows_affected > 0 {
        info!("Entry deleted successfully with ID: {}", entry_id);
        Ok(true)
    } else {
        info!("No entry found with ID: {}", entry_id);
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn setup_test_db() -> Result<(Connection, NamedTempFile)> {
        let temp_file = NamedTempFile::new()?;
        let conn = Connection::open(temp_file.path())?;
        init_database_schema(&conn)?;
        Ok((conn, temp_file))
    }

    #[test]
    fn test_create_entry_basic() -> Result<()> {
        let (conn, _temp_file) = setup_test_db()?;

        let telegram_id = 12345;
        let content = "Test ingredient list content";

        let entry_id = create_entry(&conn, telegram_id, content)?;

        // Verify the entry was created in entries table
        let mut stmt = conn.prepare("SELECT telegram_id, content FROM entries WHERE id = ?1")?;
        let mut rows = stmt.query_map(params![entry_id], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?;

        if let Some(row_result) = rows.next() {
            let (db_telegram_id, db_content) = row_result?;
            assert_eq!(db_telegram_id, telegram_id);
            assert_eq!(db_content, content);
        } else {
            panic!("Entry not found in database");
        }

        Ok(())
    }

    #[test]
    fn test_create_entry_empty_content() -> Result<()> {
        let (conn, _temp_file) = setup_test_db()?;

        let telegram_id = 12345;
        let content = "";

        let entry_id = create_entry(&conn, telegram_id, content)?;

        // Verify the entry was created
        let mut stmt = conn.prepare("SELECT telegram_id, content FROM entries WHERE id = ?1")?;
        let mut rows = stmt.query_map(params![entry_id], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?;

        if let Some(row_result) = rows.next() {
            let (db_telegram_id, db_content) = row_result?;
            assert_eq!(db_telegram_id, telegram_id);
            assert_eq!(db_content, content);
        } else {
            panic!("Entry not found in database");
        }

        Ok(())
    }

    #[test]
    fn test_create_entry_special_characters() -> Result<()> {
        let (conn, _temp_file) = setup_test_db()?;

        let telegram_id = 12345;
        let content = "!@#$%^&*()_+{}|:<>?[]\\;',./";

        let entry_id = create_entry(&conn, telegram_id, content)?;

        // Verify the entry was created
        let mut stmt = conn.prepare("SELECT telegram_id, content FROM entries WHERE id = ?1")?;
        let mut rows = stmt.query_map(params![entry_id], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?;

        if let Some(row_result) = rows.next() {
            let (db_telegram_id, db_content) = row_result?;
            assert_eq!(db_telegram_id, telegram_id);
            assert_eq!(db_content, content);
        } else {
            panic!("Entry not found in database");
        }

        Ok(())
    }

    #[test]
    fn test_create_entry_multiple_entries() -> Result<()> {
        let (conn, _temp_file) = setup_test_db()?;

        let entries = vec![
            (12345, "First ingredient list".to_string()),
            (67890, "Second ingredient list".to_string()),
            (11111, "Third ingredient list".to_string()),
        ];

        let mut entry_ids = Vec::new();

        for (telegram_id, content) in &entries {
            let entry_id = create_entry(&conn, *telegram_id, content)?;
            entry_ids.push(entry_id);
        }

        // Verify all entries were created
        for (i, expected_id) in entry_ids.iter().enumerate() {
            let (telegram_id, content) = &entries[i];
            let mut stmt =
                conn.prepare("SELECT telegram_id, content FROM entries WHERE id = ?1")?;
            let mut rows = stmt.query_map(params![expected_id], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })?;

            if let Some(row_result) = rows.next() {
                let (db_telegram_id, db_content) = row_result?;
                assert_eq!(db_telegram_id, *telegram_id);
                assert_eq!(db_content, content.clone());
            } else {
                panic!("Entry {} not found in database", expected_id);
            }
        }

        Ok(())
    }

    #[test]
    fn test_create_entry_fts_sync() -> Result<()> {
        let (conn, _temp_file) = setup_test_db()?;

        let telegram_id = 12345;
        let content = "Test content for FTS";

        let entry_id = create_entry(&conn, telegram_id, content)?;

        // Verify the entry was synced to FTS table
        let mut stmt = conn.prepare("SELECT content FROM entries_fts WHERE rowid = ?1")?;
        let mut rows = stmt.query_map(params![entry_id], |row| Ok(row.get::<_, String>(0)?))?;

        if let Some(row_result) = rows.next() {
            let fts_content = row_result?;
            assert_eq!(fts_content, content);
        } else {
            panic!("Entry not found in FTS table");
        }

        Ok(())
    }

    #[test]
    fn test_create_entry_returns_correct_id() -> Result<()> {
        let (conn, _temp_file) = setup_test_db()?;

        let telegram_id = 12345;
        let content = "Test content";

        let entry_id = create_entry(&conn, telegram_id, content)?;

        // Verify the returned ID is greater than 0
        assert!(entry_id > 0);

        // Verify the ID matches what's in the database
        let db_id: i64 = conn.query_row(
            "SELECT id FROM entries WHERE telegram_id = ?1 AND content = ?2",
            params![telegram_id, content],
            |row| row.get(0),
        )?;

        assert_eq!(entry_id, db_id);

        Ok(())
    }

    #[test]
    fn test_read_entry_exists() -> Result<()> {
        let (conn, _temp_file) = setup_test_db()?;

        let telegram_id = 12345;
        let content = "Test content for reading";
        let entry_id = create_entry(&conn, telegram_id, content)?;

        // Read the entry back
        let read_entry = read_entry(&conn, entry_id)?;

        assert!(read_entry.is_some());
        let entry = read_entry.unwrap();
        assert_eq!(entry.id, entry_id);
        assert_eq!(entry.telegram_id, telegram_id);
        assert_eq!(entry.content, content);

        Ok(())
    }

    #[test]
    fn test_read_entry_not_exists() -> Result<()> {
        let (conn, _temp_file) = setup_test_db()?;

        let entry_id = 99999; // Assuming this ID does not exist

        // Try to read a non-existing entry
        let read_entry = read_entry(&conn, entry_id)?;

        assert!(read_entry.is_none());

        Ok(())
    }

    #[test]
    fn test_read_entry_existing() -> Result<()> {
        let (conn, _temp_file) = setup_test_db()?;

        let telegram_id = 12345;
        let content = "Test ingredient list content";

        let entry_id = create_entry(&conn, telegram_id, content)?;

        let entry = read_entry(&conn, entry_id)?;

        assert!(entry.is_some());
        let entry = entry.unwrap();
        assert_eq!(entry.id, entry_id);
        assert_eq!(entry.telegram_id, telegram_id);
        assert_eq!(entry.content, content);
        assert!(!entry.created_at.is_empty());

        Ok(())
    }

    #[test]
    fn test_read_entry_nonexistent() -> Result<()> {
        let (conn, _temp_file) = setup_test_db()?;

        let entry = read_entry(&conn, 99999)?;

        assert!(entry.is_none());

        Ok(())
    }

    #[test]
    fn test_read_entry_after_multiple_creates() -> Result<()> {
        let (conn, _temp_file) = setup_test_db()?;

        let entries = vec![
            (12345, "First ingredient list".to_string()),
            (67890, "Second ingredient list".to_string()),
            (11111, "Third ingredient list".to_string()),
        ];

        let mut entry_ids = Vec::new();

        for (telegram_id, content) in &entries {
            let entry_id = create_entry(&conn, *telegram_id, content)?;
            entry_ids.push(entry_id);
        }

        // Read each entry and verify
        for (i, expected_id) in entry_ids.iter().enumerate() {
            let entry = read_entry(&conn, *expected_id)?;
            assert!(entry.is_some());
            let entry = entry.unwrap();
            assert_eq!(entry.id, *expected_id);
            assert_eq!(entry.telegram_id, entries[i].0);
            assert_eq!(entry.content, entries[i].1);
        }

        Ok(())
    }

    #[test]
    fn test_read_entry_created_at_format() -> Result<()> {
        let (conn, _temp_file) = setup_test_db()?;

        let telegram_id = 12345;
        let content = "Test content";

        let entry_id = create_entry(&conn, telegram_id, content)?;

        let entry = read_entry(&conn, entry_id)?.unwrap();

        // Verify created_at is a valid datetime string (basic check)
        assert!(entry.created_at.len() > 0);
        // SQLite datetime format should contain numbers
        assert!(entry.created_at.chars().any(|c| c.is_numeric()));

        Ok(())
    }

    #[test]
    fn test_update_entry_success() -> Result<()> {
        let (conn, _temp_file) = setup_test_db()?;

        let telegram_id = 12345;
        let content = "Initial content";
        let entry_id = create_entry(&conn, telegram_id, content)?;

        let new_content = "Updated content";
        let update_result = update_entry(&conn, entry_id, new_content)?;

        assert!(update_result);

        // Verify the entry was updated
        let updated_entry = read_entry(&conn, entry_id)?.unwrap();
        assert_eq!(updated_entry.content, new_content);

        Ok(())
    }

    #[test]
    fn test_update_entry_existing() -> Result<()> {
        let (conn, _temp_file) = setup_test_db()?;

        let telegram_id = 12345;
        let original_content = "Original ingredient list";
        let new_content = "Updated ingredient list";

        let entry_id = create_entry(&conn, telegram_id, original_content)?;

        let update_result = update_entry(&conn, entry_id, new_content)?;

        assert!(update_result);

        // Verify the entry was updated
        let updated_entry = read_entry(&conn, entry_id)?.unwrap();
        assert_eq!(updated_entry.id, entry_id);
        assert_eq!(updated_entry.telegram_id, telegram_id);
        assert_eq!(updated_entry.content, new_content);

        Ok(())
    }

    #[test]
    fn test_update_entry_nonexistent() -> Result<()> {
        let (conn, _temp_file) = setup_test_db()?;

        let update_result = update_entry(&conn, 99999, "Some content")?;

        assert!(!update_result);

        Ok(())
    }

    #[test]
    fn test_update_entry_empty_content() -> Result<()> {
        let (conn, _temp_file) = setup_test_db()?;

        let telegram_id = 12345;
        let original_content = "Original content";
        let new_content = "";

        let entry_id = create_entry(&conn, telegram_id, original_content)?;

        let update_result = update_entry(&conn, entry_id, new_content)?;

        assert!(update_result);

        // Verify the entry was updated to empty content
        let updated_entry = read_entry(&conn, entry_id)?.unwrap();
        assert_eq!(updated_entry.content, new_content);

        Ok(())
    }

    #[test]
    fn test_update_entry_special_characters() -> Result<()> {
        let (conn, _temp_file) = setup_test_db()?;

        let telegram_id = 12345;
        let original_content = "Original content";
        let new_content = "!@#$%^&*()_+{}|:<>?[]\\;',./";

        let entry_id = create_entry(&conn, telegram_id, original_content)?;

        let update_result = update_entry(&conn, entry_id, new_content)?;

        assert!(update_result);

        // Verify the entry was updated with special characters
        let updated_entry = read_entry(&conn, entry_id)?.unwrap();
        assert_eq!(updated_entry.content, new_content);

        Ok(())
    }

    #[test]
    fn test_update_entry_fts_sync() -> Result<()> {
        let (conn, _temp_file) = setup_test_db()?;

        let telegram_id = 12345;
        let original_content = "Original content for FTS";
        let new_content = "Updated content for FTS";

        let entry_id = create_entry(&conn, telegram_id, original_content)?;

        let update_result = update_entry(&conn, entry_id, new_content)?;

        assert!(update_result);

        // Verify the FTS table was updated via trigger
        let mut stmt = conn.prepare("SELECT content FROM entries_fts WHERE rowid = ?1")?;
        let fts_content: String = stmt.query_row(params![entry_id], |row| row.get(0))?;

        assert_eq!(fts_content, new_content);

        Ok(())
    }

    #[test]
    fn test_update_entry_multiple_updates() -> Result<()> {
        let (conn, _temp_file) = setup_test_db()?;

        let telegram_id = 12345;
        let content1 = "First version";
        let content2 = "Second version";
        let content3 = "Third version";

        let entry_id = create_entry(&conn, telegram_id, content1)?;

        // First update
        let update_result1 = update_entry(&conn, entry_id, content2)?;
        assert!(update_result1);

        let entry = read_entry(&conn, entry_id)?.unwrap();
        assert_eq!(entry.content, content2);

        // Second update
        let update_result2 = update_entry(&conn, entry_id, content3)?;
        assert!(update_result2);

        let entry = read_entry(&conn, entry_id)?.unwrap();
        assert_eq!(entry.content, content3);

        Ok(())
    }

    #[test]
    fn test_delete_entry_success() -> Result<()> {
        let (conn, _temp_file) = setup_test_db()?;

        let telegram_id = 12345;
        let content = "Content to be deleted";
        let entry_id = create_entry(&conn, telegram_id, content)?;

        let delete_result = delete_entry(&conn, entry_id)?;

        assert!(delete_result);

        // Verify the entry was deleted
        let read_entry = read_entry(&conn, entry_id)?;
        assert!(read_entry.is_none());

        Ok(())
    }

    #[test]
    fn test_delete_entry_existing() -> Result<()> {
        let (conn, _temp_file) = setup_test_db()?;

        let telegram_id = 12345;
        let content = "Content to be deleted";

        let entry_id = create_entry(&conn, telegram_id, content)?;

        // Verify entry exists before deletion
        let entry_before = read_entry(&conn, entry_id)?;
        assert!(entry_before.is_some());

        // Delete the entry
        let delete_result = delete_entry(&conn, entry_id)?;

        assert!(delete_result);

        // Verify entry no longer exists
        let entry_after = read_entry(&conn, entry_id)?;
        assert!(entry_after.is_none());

        Ok(())
    }

    #[test]
    fn test_delete_entry_nonexistent() -> Result<()> {
        let (conn, _temp_file) = setup_test_db()?;

        let delete_result = delete_entry(&conn, 99999)?;

        assert!(!delete_result);

        Ok(())
    }

    #[test]
    fn test_delete_entry_fts_sync() -> Result<()> {
        let (conn, _temp_file) = setup_test_db()?;

        let telegram_id = 12345;
        let content = "Content for deletion sync test";

        let entry_id = create_entry(&conn, telegram_id, content)?;

        // Verify FTS entry exists before deletion
        let mut stmt = conn.prepare("SELECT content FROM entries_fts WHERE rowid = ?1")?;
        let fts_result_before = stmt.query_row(params![entry_id], |row| row.get::<_, String>(0));
        assert!(fts_result_before.is_ok());

        // Delete the entry
        let delete_result = delete_entry(&conn, entry_id)?;

        assert!(delete_result);

        // Verify the entry was removed from FTS table
        let mut stmt = conn.prepare("SELECT content FROM entries_fts WHERE rowid = ?1")?;
        let fts_result_after = stmt.query_row(params![entry_id], |row| row.get::<_, String>(0));

        assert!(fts_result_after.is_err()); // Should be an error since the entry was deleted

        Ok(())
    }

    #[test]
    fn test_delete_entry_multiple_deletions() -> Result<()> {
        let (conn, _temp_file) = setup_test_db()?;

        let entries = vec![
            (12345, "First entry".to_string()),
            (67890, "Second entry".to_string()),
            (11111, "Third entry".to_string()),
        ];

        let mut entry_ids = Vec::new();

        // Create multiple entries
        for (telegram_id, content) in &entries {
            let entry_id = create_entry(&conn, *telegram_id, content)?;
            entry_ids.push(entry_id);
        }

        // Delete each entry and verify
        for entry_id in entry_ids {
            let delete_result = delete_entry(&conn, entry_id)?;
            assert!(delete_result);

            // Verify entry no longer exists
            let entry = read_entry(&conn, entry_id)?;
            assert!(entry.is_none());
        }

        Ok(())
    }

    #[test]
    fn test_delete_entry_same_id_twice() -> Result<()> {
        let (conn, _temp_file) = setup_test_db()?;

        let telegram_id = 12345;
        let content = "Content for double deletion test";

        let entry_id = create_entry(&conn, telegram_id, content)?;

        // First deletion should succeed
        let delete_result1 = delete_entry(&conn, entry_id)?;
        assert!(delete_result1);

        // Second deletion should fail
        let delete_result2 = delete_entry(&conn, entry_id)?;
        assert!(!delete_result2);

        Ok(())
    }
}
