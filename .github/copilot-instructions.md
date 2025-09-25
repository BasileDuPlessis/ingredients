# Ingredients Telegram Bot - AI Coding Guidelines

## Project Overview
This is a Telegram bot that extracts text from images using OCR (Optical Character Recognition). It processes photos and image files sent by users, extracts text using Tesseract, and stores the results in a SQLite database with full-text search capabilities.

## Architecture Components

### Core Modules
- **`main.rs`**: Application entry point, initializes services and starts the bot dispatcher
- **`bot.rs`**: Telegram message handling, image download/processing, user interaction logic
- **`ocr.rs`**: OCR processing with Tesseract, circuit breaker pattern, format validation, instance management
- **`db.rs`**: SQLite database operations with FTS (Full-Text Search) support
- **`localization.rs`**: Internationalization using Fluent bundles (English/French)

### Key Dependencies
- **teloxide**: Telegram bot framework
- **leptess**: Tesseract OCR Rust bindings
- **rusqlite**: SQLite database access
- **fluent-bundle**: Internationalization framework
- **tokio**: Async runtime
- **anyhow**: Error handling

## Critical Developer Workflows

### Environment Setup
```bash
# Required environment variables
TELEGRAM_BOT_TOKEN=your_bot_token_here
DATABASE_URL=ingredients.db

# Optional
HEALTH_PORT=8080
```

### Build & Run
```bash
cargo build                    # Debug build
cargo build --release         # Optimized build
cargo run                     # Run with environment variables
cargo test                    # Run test suite
```

### Database Management
- Schema auto-initializes on startup via `db::init_database_schema()`
- Uses SQLite with FTS virtual table for content search
- Triggers maintain FTS table synchronization

## Project-Specific Patterns & Conventions

### OCR Processing Architecture
- **Circuit Breaker Pattern**: Prevents cascading failures during OCR operations
  - Configured in `RecoveryConfig` with threshold and reset timeout
  - Records failures/successes to protect system stability
- **Instance Reuse**: OCR instances cached by language combination for performance
  - Managed by `OcrInstanceManager` with `Arc<Mutex<LepTess>>`
  - Eliminates Tesseract initialization overhead (~100-500ms per instance)
- **Format-Specific Validation**: Different size limits per image format
  - PNG: 15MB, JPEG: 10MB, BMP: 5MB, TIFF: 20MB
  - Memory usage estimation before processing
- **Retry Logic**: Exponential backoff with jitter (3 retries, 1s-10s delays)

### Error Handling Strategy
- **Custom Error Types**: `OcrError` enum with specific variants (Validation, Timeout, etc.)
- **User-Friendly Messages**: Localized error responses via Fluent bundles
- **Graceful Degradation**: Circuit breaker prevents system overload
- **Resource Cleanup**: Temporary files always removed after processing

### Database Design
- **Simple Schema**: Single `entries` table with FTS virtual table
- **Auto-Sync Triggers**: INSERT/UPDATE/DELETE triggers maintain FTS index
- **User Isolation**: Entries filtered by `telegram_id`

### Internationalization Approach
- **Fluent Framework**: Uses `.ftl` files in `locales/{lang}/main.ftl`
- **Language Detection**: Based on Telegram `user.language_code`
- **Fallback Strategy**: Unsupported languages default to English
- **Message Keys**: Descriptive keys like `error-ocr-timeout`, `success-extraction`

### Async Patterns
- **Shared State**: Database connection wrapped in `Arc<Mutex<>>`
- **Timeout Protection**: 30-second timeouts on OCR operations
- **Background Processing**: Image downloads and OCR run asynchronously

## Integration Points & External Dependencies

### Telegram Bot API
- **Message Types**: Handles text, photos, documents, unsupported messages
- **File Downloads**: Downloads via Telegram API with authentication
- **User Context**: Extracts language codes for localization

### OCR Engine (Tesseract)
- **Language Support**: Configured for English + French (`eng+fra`)
- **Image Formats**: PNG, JPEG, BMP, TIFF with format detection
- **Performance Optimization**: Instance pooling and reuse

### File System Operations
- **Temporary Files**: Images downloaded to temp files, always cleaned up
- **Path Handling**: Absolute paths required for Tesseract operations

## Code Quality Standards

### Testing Approach
- **Unit Tests**: Comprehensive coverage for all modules
- **Integration Tests**: Database operations, OCR validation
- **Mock Data**: Temporary files and in-memory databases for testing

### Security Considerations
- **Input Validation**: File size limits, format restrictions
- **Resource Limits**: Memory estimation, timeout protection
- **Audit Configuration**: `deny.toml` for dependency security scanning

### Performance Optimizations
- **Connection Reuse**: Single database connection shared across requests
- **Instance Caching**: OCR instances reused to reduce initialization time
- **Format Pre-validation**: Quick rejection of unsupported/oversized files

## Linting, Formatting, and Code Review Standards
- **Clippy Enforcement**: All code must pass `cargo clippy` with no warnings. Use the default Clippy lints and fix all issues before submitting code or pull requests.
    - For new code, run:  
      ```bash
      cargo clippy --all-targets --all-features -- -D warnings
      ```
    - If you encounter Clippy lints that are not applicable, document and justify any allowed exceptions with inline comments.

- **Rustfmt Enforcement**: All code must be formatted with `rustfmt` using the default Rust style.
    - Run:  
      ```bash
      cargo fmt --all -- --check
      ```
    - CI and all contributors must ensure code formatting matches the output of `cargo fmt`.

- **CI Integration**: PRs may be rejected if they do not pass both Clippy and formatting checks.
- **Copilot and AI Contributions**: AI-generated code must always meet the above lint and formatting standards, without exception.


## Common Development Tasks

### Adding New Features
1. **Database Changes**: Update schema in `db.rs`, add migration logic
2. **New Commands**: Add handlers in `bot.rs` message processing logic
3. **OCR Enhancements**: Modify `ocr.rs` with new validation or processing logic
4. **Localization**: Add keys to `.ftl` files, update `localization.rs` if needed

### Debugging Issues
- **OCR Failures**: Check circuit breaker state, Tesseract logs, temp file cleanup
- **Database Issues**: Verify FTS triggers, connection sharing, schema initialization
- **Localization**: Confirm language detection, Fluent bundle loading

### Deployment Considerations
- **Environment Variables**: Secure token storage, database path configuration
- **Resource Limits**: Monitor memory usage, file system space for temp files
- **Monitoring**: Circuit breaker metrics, OCR success/failure rates</content>
<parameter name="filePath">/Users/basile.du.plessis/Documents/ingredients/.github/copilot-instructions.md