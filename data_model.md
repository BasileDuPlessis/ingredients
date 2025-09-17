# Data Model for Ingredients Telegram Bot

## Overview
This document defines the database schema for the Ingredients Telegram bot, which stores both raw extracted text and structured ingredient data from ingredient list images and user comments in searchable tables.

**Note**: For detailed information about the structured ingredient and quantity extraction data model, see [INGREDIENT_DATA_MODEL.md](INGREDIENT_DATA_MODEL.md).

## Entities and Tables

### 1. Entries Table (Legacy)
Stores raw extracted text and comments from Telegram messages.

| Column      | Type    | Constraints          | Description                  |
|-------------|---------|----------------------|------------------------------|
| id          | INTEGER | PRIMARY KEY AUTOINCREMENT | Unique identifier            |
| telegram_id | INTEGER | NOT NULL             | Telegram user ID             |
| content     | TEXT    | NOT NULL             | Extracted text and comments  |
| created_at  | DATETIME| DEFAULT CURRENT_TIMESTAMP | Entry creation timestamp     |

### 2. Ingredient Entries Table (New)
Stores structured ingredient data with parsed quantities and units.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| id | INTEGER | PRIMARY KEY AUTOINCREMENT | Unique identifier |
| telegram_id | INTEGER | NOT NULL | Telegram user ID |
| original_text | TEXT | NOT NULL | Original OCR text |
| parsed_ingredients | TEXT | NOT NULL | JSON serialized IngredientList |
| parsing_confidence | REAL | NOT NULL DEFAULT 0.0 | Overall parsing confidence |
| created_at | DATETIME | DEFAULT CURRENT_TIMESTAMP | Entry creation timestamp |

## Full-Text Search
Both tables support SQLite's Full-Text Search (FTS) for efficient querying:

### Legacy Text Search
```sql
CREATE VIRTUAL TABLE entries_fts USING fts5(content, content='entries', content_rowid='id');
```

### Structured Ingredient Search
```sql
CREATE VIRTUAL TABLE ingredient_entries_fts USING fts5(original_text, content='ingredient_entries', content_rowid='id');
```

## Data Flow

### Processing Pipeline
1. **OCR Extraction**: Raw text extracted from ingredient list images
2. **Structured Parsing**: Text processed into `IngredientList` with individual `Ingredient` objects
3. **Storage**: Both original text and structured data stored in `ingredient_entries` table
4. **Search**: Full-text search available on original text; structured queries possible on parsed data

### Structured Data Format
The `parsed_ingredients` column contains JSON-serialized `IngredientList` objects with:
- Individual ingredients with names, quantities, and modifiers
- Quantity types: exact amounts, fractions, ranges, ambiguous quantities
- Units: volume, weight, count, and specialized units
- Confidence scores for parsing accuracy
- Unparsed lines for fallback handling

## Relationships
- No complex relationships; each entry is standalone
- Multiple entries can belong to the same user (telegram_id)
- Both legacy and structured entries coexist

## Sample Data

### Legacy Entry
- id: 1, telegram_id: 123456789, content: "2 cups flour\n1 cup sugar\n3 eggs", created_at: 2025-09-05 12:00:00

### Structured Entry
- id: 1, telegram_id: 123456789, original_text: "2 cups flour\n1 tbsp salt\n1/2 tsp pepper"
- parsed_ingredients: JSON containing structured ingredient data with quantities and units
- parsing_confidence: 0.95

## API Functions

### Structured Ingredient Operations
- `create_ingredient_entry()` - Store parsed ingredient data
- `read_ingredient_entry()` - Retrieve structured entry by ID
- `get_parsed_ingredients()` - Deserialize JSON to IngredientList
- `search_ingredient_entries()` - Full-text search on ingredient entries

### Legacy Operations (preserved)
- `_create_entry()` - Store raw text (legacy)
- `_read_entry()` - Retrieve raw text entry
- `_update_entry()` - Update raw text content
- `_delete_entry()` - Remove raw text entry

## Migration Strategy
- **Backward Compatibility**: Legacy `entries` table and functions remain functional
- **Dual Storage**: New functionality uses structured format while maintaining text fallback
- **Gradual Migration**: Existing data remains in legacy format; new data uses structured format
- **Search Federation**: Both search systems work independently and can be combined

## Implementation Notes
- Tables are auto-created at application startup using `CREATE TABLE IF NOT EXISTS`
- FTS virtual tables created with triggers to keep them synchronized
- Database path loaded from `DATABASE_URL` environment variable
- JSON serialization handles complex ingredient structures with proper error handling
- Confidence scoring helps identify parsing quality and potential issues

## Future Enhancements
- Unit conversion and normalization
- Recipe scaling based on structured quantities
- Nutritional analysis integration
- Advanced search with quantity filters
- Machine learning improvements based on user feedback

For complete details on the structured data model, parsing logic, and examples, see [INGREDIENT_DATA_MODEL.md](INGREDIENT_DATA_MODEL.md).
