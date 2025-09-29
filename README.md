# Ingredients Telegram Bot

A Telegram bot that extracts text from images using OCR (Optical Character Recognition) and stores ingredient lists in a searchable database.

## Features

- **OCR Text Extraction**: Uses Tesseract OCR to extract text from images and photos
- **Ingredient Parsing**: Automatically detects and parses measurements and ingredients from recipe text
- **Quantity-Only Support**: Recognizes ingredients with quantities but no measurement units (e.g., "6 oeufs", "4 pommes")
- **Full-Text Search**: PostgreSQL full-text search for efficient content searching
- **Multilingual Support**: English and French language support with localized messages
- **Circuit Breaker Pattern**: Protects against OCR failures with automatic recovery
- **Database Storage**: Persistent storage of extracted text and user interactions

## Supported Measurement Formats

### Traditional Measurements
- Volume: `2 cups flour`, `1 tablespoon sugar`, `250 ml milk`
- Weight: `500g butter`, `1 kg tomatoes`, `2 lbs beef`
- Count: `3 eggs`, `2 slices bread`, `1 can tomatoes`

### Quantity-Only Ingredients
- French: `6 oeufs`, `4 pommes`, `3 carottes`
- English: `5 apples`, `2 onions`, `8 potatoes`

## Installation

### Prerequisites
- Rust 1.70+
- Tesseract OCR with English and French language packs
- PostgreSQL database

### Setup
1. Clone the repository:
   ```bash
   git clone https://github.com/BasileDuPlessis/ingredients.git
   cd ingredients
   ```

2. Install dependencies:
   ```bash
   cargo build
   ```

3. Set up environment variables:
   ```bash
   cp .env.example .env
   # Edit .env with your Telegram bot token
   ```

4. Run the bot:
   ```bash
   cargo run
   ```

## Configuration

### Environment Variables
- `TELEGRAM_BOT_TOKEN`: Your Telegram bot token from @BotFather
- `DATABASE_URL`: PostgreSQL database connection string (e.g., `postgresql://user:pass@localhost/db`)
- `HEALTH_PORT`: Optional health check port (default: 8080)

### OCR Configuration
- **Languages**: English + French (`eng+fra`)
- **File Size Limits**: PNG: 15MB, JPEG: 10MB, BMP: 5MB, TIFF: 20MB
- **Timeout**: 30 seconds per OCR operation
- **Circuit Breaker**: 3 failures trigger, 60-second reset timeout

## Usage

1. Start a chat with your bot on Telegram
2. Send an image containing an ingredient list or recipe
3. The bot will:
   - Download and process the image
   - Extract text using OCR
   - Parse measurements and ingredients
   - Store the results in the database
   - Confirm successful processing

### Example Interactions

**Input Image:**
```
Crêpes Suzette

Ingrédients:
125 g de farine
2 œufs
1/2 litre de lait
2 cuillères à soupe de sucre
```

**Bot Response:**
Found 4 measurements:
1. 125 g → "farine"
2. 2 → "œufs" (quantity-only)
3. 1/2 litre → "lait"
4. 2 cuillères à soupe → "sucre"

## Architecture

### Core Modules
- **`main.rs`**: Application entry point and Telegram bot dispatcher
- **`bot.rs`**: Message handling, image processing, and user interactions
- **`ocr.rs`**: Tesseract OCR integration with circuit breaker pattern
- **`db.rs`**: PostgreSQL database operations with full-text search support
- **`text_processing.rs`**: Measurement detection and ingredient parsing
- **`localization.rs`**: Internationalization support (English/French)

### Key Dependencies
- `teloxide`: Telegram bot framework
- `leptess`: Tesseract OCR Rust bindings
- `sqlx`: PostgreSQL database access
- `fluent-bundle`: Internationalization framework
- `tokio`: Async runtime

## Development

### Building
```bash
cargo build                    # Debug build
cargo build --release         # Optimized release build
```

### Testing
```bash
cargo test                     # Run all tests
cargo test --doc              # Run documentation tests
cargo run --example recipe_parser  # Run recipe parsing example
```

### Code Quality
- **Linting**: `cargo clippy` (all warnings must pass)
- **Formatting**: `cargo fmt` (must match standard Rust formatting)
- **Security**: `cargo deny` for dependency security auditing

## Examples

See the `examples/` directory for usage examples:

- `recipe_parser.rs`: Comprehensive recipe parsing demonstration
- Shows both traditional measurements and quantity-only ingredients
- Demonstrates configuration options and post-processing

## Database Schema

The bot uses a PostgreSQL schema with full-text search support:

```sql
-- Users table: Maps Telegram IDs to internal IDs and tracks language preference
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    telegram_id BIGINT UNIQUE NOT NULL,
    language_code VARCHAR(10) DEFAULT 'en',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- OCR entries table: Stores full OCR text blocks for audit/traceability
CREATE TABLE ocr_entries (
    id SERIAL PRIMARY KEY,
    telegram_id BIGINT NOT NULL,
    content TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    content_tsv tsvector GENERATED ALWAYS AS (to_tsvector('english', content)) STORED
);

-- Ingredients table: Links to users and optionally to OCR entries, stores parsed data
CREATE TABLE ingredients (
    id SERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id),
    ocr_entry_id BIGINT REFERENCES ocr_entries(id),
    name VARCHAR(255) NOT NULL,
    quantity DECIMAL(10,3),
    unit VARCHAR(50),
    raw_text TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id),
    FOREIGN KEY (ocr_entry_id) REFERENCES ocr_entries(id)
);

-- Indexes for performance
CREATE INDEX ocr_entries_content_tsv_idx ON ocr_entries USING GIN (content_tsv);
CREATE INDEX ingredients_user_id_idx ON ingredients(user_id);
CREATE INDEX ingredients_ocr_entry_id_idx ON ingredients(ocr_entry_id);
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Ensure all tests pass: `cargo test`
6. Format code: `cargo fmt`
7. Lint code: `cargo clippy`
8. Commit your changes
9. Push to your fork
10. Create a pull request

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Changelog

### v0.1.1 (2025-09-29)
- **Removed**: Conversion ratios table and related functionality
- **Refactored**: Measurement units moved to external JSON configuration (`config/measurement_units.json`)
- **Updated**: Database schema simplified to 3 core tables (users, ocr_entries, ingredients)
- **Improved**: Code cleanup and removal of unused imports
- **Fixed**: Clippy warnings and placeholder tests

### v0.1.0 (2025-09-26)
- Initial release with OCR text extraction and ingredient parsing
- Support for traditional measurements (cups, grams, liters, etc.)
- **New**: Quantity-only ingredient support (e.g., "6 oeufs", "4 pommes")
- PostgreSQL database with full-text search
- English and French localization
- Circuit breaker pattern for OCR reliability
- Telegram bot integration</content>
<parameter name="filePath">/Users/basile.du.plessis/Documents/ingredients/README.md