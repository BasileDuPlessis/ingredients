# Data Model for Ingredients Telegram Bot

## Overview
This document defines the comprehensive PostgreSQL database schema for the Ingredients Telegram bot. The schema supports user management, OCR text extraction, ingredient parsing, and unit conversions with full-text search capabilities.

## Architecture Principles

- **Normalized Design**: Proper separation of concerns with related entities
- **Audit Trail**: Full OCR text preservation for traceability
- **User Isolation**: Multi-user support with proper data segregation
- **Performance**: Indexed queries with full-text search optimization
- **Extensibility**: Flexible schema for future feature additions

## Entities and Tables

### 1. Users Table
Manages Telegram user accounts and preferences.

| Column       | Type          | Constraints                    | Description                          |
|--------------|---------------|-------------------------------|--------------------------------------|
| id           | SERIAL        | PRIMARY KEY                   | Internal user identifier             |
| telegram_id  | BIGINT        | UNIQUE NOT NULL               | Telegram user ID                     |
| language_code| VARCHAR(10)   | DEFAULT 'en'                  | User language preference (en/fr)     |
| created_at   | TIMESTAMP     | DEFAULT CURRENT_TIMESTAMP     | Account creation timestamp           |
| updated_at   | TIMESTAMP     | DEFAULT CURRENT_TIMESTAMP     | Last update timestamp                |

**Indexes:**
- Primary key on `id`
- Unique index on `telegram_id`

### 2. OCR Entries Table
Stores complete OCR-extracted text for audit and search purposes.

| Column       | Type          | Constraints                    | Description                          |
|--------------|---------------|-------------------------------|--------------------------------------|
| id           | SERIAL        | PRIMARY KEY                   | OCR entry identifier                 |
| telegram_id  | BIGINT        | NOT NULL                      | Telegram user ID (for filtering)     |
| content      | TEXT          | NOT NULL                      | Full OCR-extracted text              |
| created_at   | TIMESTAMP     | DEFAULT CURRENT_TIMESTAMP     | OCR processing timestamp             |
| content_tsv  | tsvector      | GENERATED ALWAYS AS (to_tsvector('english', content)) STORED | Full-text search vector |

**Indexes:**
- Primary key on `id`
- GIN index on `content_tsv` for full-text search
- Index on `telegram_id` for user filtering

### 3. Ingredients Table
Stores parsed ingredient data with optional OCR linkage.

| Column       | Type          | Constraints                    | Description                          |
|--------------|---------------|-------------------------------|--------------------------------------|
| id           | SERIAL        | PRIMARY KEY                   | Ingredient identifier                |
| user_id      | BIGINT        | NOT NULL REFERENCES users(id) | Owner user ID                        |
| ocr_entry_id | BIGINT        | REFERENCES ocr_entries(id)    | Source OCR entry (optional)          |
| name         | VARCHAR(255)  | NOT NULL                      | Ingredient name                      |
| quantity     | DECIMAL(10,3) | NULL                          | Parsed quantity value                |
| unit         | VARCHAR(50)   | NULL                          | Measurement unit                     |
| raw_text     | TEXT          | NOT NULL                      | Original parsed text                 |
| created_at   | TIMESTAMP     | DEFAULT CURRENT_TIMESTAMP     | Creation timestamp                   |
| updated_at   | TIMESTAMP     | DEFAULT CURRENT_TIMESTAMP     | Last update timestamp                |

**Indexes:**
- Primary key on `id`
- Foreign key indexes on `user_id` and `ocr_entry_id`

### 4. Conversion Ratios Table
Stores ingredient-specific unit conversion factors.

| Column          | Type          | Constraints                    | Description                          |
|--------------   |---------------|-------------------------------|--------------------------------------|
| id              | SERIAL        | PRIMARY KEY                   | Conversion ratio identifier          |
| ingredient_name | VARCHAR(255)  | NOT NULL                      | Ingredient name                      |
| from_unit       | VARCHAR(50)   | NOT NULL                      | Source unit                          |
| to_unit         | VARCHAR(50)   | NOT NULL                      | Target unit                          |
| ratio           | DECIMAL(10,6) | NOT NULL                      | Conversion multiplier                |
| created_at      | TIMESTAMP     | DEFAULT CURRENT_TIMESTAMP     | Creation timestamp                   |
| updated_at      | TIMESTAMP     | DEFAULT CURRENT_TIMESTAMP     | Last update timestamp                |

**Constraints:**
- Unique constraint on `(ingredient_name, from_unit, to_unit)`

**Indexes:**
- Primary key on `id`
- Index on `ingredient_name` for ingredient filtering

## Relationships

### Entity Relationships
```
Users (1) ──── (N) Ingredients
Users (1) ──── (N) OCR Entries
OCR Entries (1) ──── (0..1) Ingredients
Ingredients (N) ──── (1) Conversion Ratios (by ingredient_name)
```

### Foreign Key Constraints
- `ingredients.user_id` → `users.id` (CASCADE)
- `ingredients.ocr_entry_id` → `ocr_entries.id` (SET NULL)
- All relationships maintain referential integrity

## Full-Text Search Implementation

### PostgreSQL FTS Setup
```sql
-- Generated column for automatic FTS vector creation
content_tsv tsvector GENERATED ALWAYS AS (to_tsvector('english', content)) STORED

-- GIN index for efficient FTS queries
CREATE INDEX ocr_entries_content_tsv_idx ON ocr_entries USING GIN (content_tsv);
```

### Search Queries
```sql
-- Basic full-text search
SELECT * FROM ocr_entries
WHERE telegram_id = $1 AND content_tsv @@ plainto_tsquery('english', $2);

-- Ranked search results
SELECT *, ts_rank(content_tsv, query) as rank
FROM ocr_entries, plainto_tsquery('english', $2) as query
WHERE telegram_id = $1 AND content_tsv @@ query
ORDER BY rank DESC;
```

## Data Flow and Usage Patterns

### OCR Processing Flow
1. **Image Reception**: User sends image to Telegram bot
2. **OCR Processing**: Tesseract extracts text from image
3. **Entry Creation**: Full text stored in `ocr_entries` table
4. **Parsing**: Text analyzed for ingredients and measurements
5. **Ingredient Storage**: Parsed data stored in `ingredients` table
6. **User Association**: All data linked to authenticated user

### Query Patterns

#### User-Specific Queries
```sql
-- Get all ingredients for a user
SELECT * FROM ingredients WHERE user_id = $1 ORDER BY created_at DESC;

-- Get recent OCR entries for a user
SELECT * FROM ocr_entries WHERE telegram_id = $1 ORDER BY created_at DESC LIMIT 10;
```

#### Search Operations
```sql
-- Full-text search in OCR content
SELECT e.*, ts_rank(e.content_tsv, q.query) as rank
FROM ocr_entries e, plainto_tsquery('english', $2) q
WHERE e.telegram_id = $1 AND e.content_tsv @@ q.query
ORDER BY rank DESC;

-- Ingredient search with conversions
SELECT i.*, cr.ratio
FROM ingredients i
LEFT JOIN conversion_ratios cr ON i.name = cr.ingredient_name
WHERE i.user_id = $1 AND i.name ILIKE $2;
```

## Sample Data

### Users Table
```sql
INSERT INTO users (telegram_id, language_code) VALUES (123456789, 'fr');
-- Result: id=1, telegram_id=123456789, language_code='fr'
```

### OCR Entries Table
```sql
INSERT INTO ocr_entries (telegram_id, content) VALUES (123456789, '2 cups flour\n1 cup sugar\n3 eggs');
-- Result: id=1, telegram_id=123456789, content='2 cups flour\n1 cup sugar\n3 eggs'
```

### Ingredients Table
```sql
INSERT INTO ingredients (user_id, ocr_entry_id, name, quantity, unit, raw_text)
VALUES (1, 1, 'flour', 2.0, 'cups', '2 cups flour');
-- Links to user and OCR entry for full traceability
```

### Conversion Ratios Table
```sql
INSERT INTO conversion_ratios (ingredient_name, from_unit, to_unit, ratio)
VALUES ('flour', 'cup', 'gram', 120.0);
-- Allows converting flour measurements between cups and grams
```

## Performance Considerations

### Indexing Strategy
- **GIN Indexes**: For full-text search vectors
- **Foreign Key Indexes**: For relationship traversal
- **Partial Indexes**: For common query patterns

### Query Optimization
- Use `EXPLAIN ANALYZE` for complex queries
- Consider partitioning for large datasets
- Monitor slow queries in production

### Data Volume Estimates
- **Users**: Thousands to millions
- **OCR Entries**: Millions (one per image)
- **Ingredients**: Tens of millions (multiple per entry)
- **Conversions**: Hundreds (per ingredient type)

## Migration and Schema Evolution

### Schema Initialization
- Auto-creation on application startup
- `CREATE TABLE IF NOT EXISTS` for safety
- Version checking for future migrations

### Backward Compatibility
- Existing data preserved during schema updates
- Optional fields allow gradual data population
- Foreign key constraints prevent orphaned records

## Security and Data Privacy

### User Data Isolation
- All queries filtered by `user_id` or `telegram_id`
- No cross-user data access
- Telegram ID validation at API level

### Data Retention
- Full OCR text preserved for audit trails
- Configurable retention policies
- GDPR compliance considerations

## Future Extensions

### Potential Enhancements
- **Recipe Templates**: Store complete recipes
- **Shopping Lists**: Generated from ingredients
- **Nutritional Data**: Integration with nutrition APIs
- **Meal Planning**: Recipe scheduling and planning
- **Social Features**: Recipe sharing between users

### Schema Extensions
- Additional metadata fields
- Recipe categorization
- User preferences and dietary restrictions
- Integration with external recipe databases

## Implementation Notes

### Database Connection
- Connection pooling via `sqlx::PgPool`
- Environment-based configuration (`DATABASE_URL`)
- Connection health monitoring

### Error Handling
- Comprehensive error types for each operation
- Graceful degradation on database failures
- Transaction management for data consistency

### Testing Strategy
- Unit tests for individual operations
- Integration tests with test database
- Conditional test execution (skips without DB)

This schema provides a solid foundation for the Ingredients bot while maintaining flexibility for future enhancements.
