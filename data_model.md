# Data Model for Ingredients Telegram Bot

## Overview
This document defines a simple database schema for the Ingredients Telegram bot, which stores extracted text from ingredient list images and user comments in a searchable table.

## Entities and Tables

### 1. Entries Table
Stores all extracted text and comments from Telegram messages.

| Column      | Type    | Constraints          | Description                  |
|-------------|---------|----------------------|------------------------------|
| id          | INTEGER | PRIMARY KEY AUTOINCREMENT | Unique identifier            |
| telegram_id | INTEGER | NOT NULL             | Telegram user ID             |
| content     | TEXT    | NOT NULL             | Extracted text and comments  |
| created_at  | DATETIME| DEFAULT CURRENT_TIMESTAMP | Entry creation timestamp     |

## Full-Text Search
To enable searching within the content field, use SQLite's Full-Text Search (FTS) virtual table:

```sql
CREATE VIRTUAL TABLE entries_fts USING fts5(content, content='entries', content_rowid='id');
```

This allows efficient full-text queries on the content field.

## Relationships
- No complex relationships; each entry is standalone
- Multiple entries can belong to the same user (telegram_id)

## Sample Data
- id: 1, telegram_id: 123456789, content: "2 cups flour\n1 cup sugar\n3 eggs", created_at: 2025-09-05 12:00:00
- id: 2, telegram_id: 123456789, content: "Great recipe for cake!", created_at: 2025-09-05 12:01:00

## Notes
- Use FTS for searching the content field
- telegram_id can be used to filter entries by user
- Keep it simple for initial implementation

## Implementation Notes
- Tables are auto-created at application startup using `CREATE TABLE IF NOT EXISTS`
- FTS virtual table is created with triggers to keep it synchronized
- Database path is loaded from `DATABASE_URL` environment variable
