# Ingredients Rust Application

Ingredients is a Telegram bot application built in Rust that receives pictures of ingredient lists for recipes and comments from users, extracts text from images using OCR (Optical Character Recognition), and stores all the extracted text and comments in a full-text searchable database table. Users can later query the stored content through the Telegram interface.

Always reference these instructions first and fallback to search or bash commands only when you encounter unexpected information that does not match the info here.

## Working Effectively

### Prerequisites
- Rust toolchain is already installed (rustc 1.89.0, cargo 1.89.0)
- **CRITICAL SYSTEM DEPENDENCIES REQUIRED**: Must install OCR system libraries first:
  ```bash
  sudo apt update && sudo apt install -y tesseract-ocr tesseract-ocr-eng tesseract-ocr-fra libleptonica-dev libtesseract-dev pkg-config
  ```
- **Environment setup**: Copy `.env.example` to `.env` and set required variables:
  - `TELEGRAM_BOT_TOKEN=your_bot_token_here`
  - `DATABASE_URL=ingredients.db` (SQLite database path)

### Essential Development Commands
- **Check code without building**: `cargo check` -- takes ~26 seconds first time, ~1 second incremental. NEVER CANCEL: Use timeout 120+ seconds.
- **Build the project**: `cargo build` -- takes ~51 seconds for first build, ~2-3 seconds for incremental builds. NEVER CANCEL: Use timeout 300+ seconds.
- **Build for release**: `cargo build --release` -- takes ~115 seconds (2 minutes). NEVER CANCEL: Use timeout 600+ seconds.
- **Run tests**: `cargo test` -- takes ~4 seconds (63 tests exist). NEVER CANCEL: Use timeout 120+ seconds.
- **Run the application**: `cargo run` -- takes ~0.1 seconds to run after build
- **Format code**: `cargo fmt` -- takes ~0.3 seconds for formatting check
- **Lint code**: `cargo clippy --all-targets --all-features -- -D warnings` -- takes ~2 seconds for linting analysis
- **Generate documentation**: `cargo doc --no-deps` -- takes ~3 seconds
- **Clean build artifacts**: `cargo clean` -- removes target/ directory

### Development Workflow
1. **Always start with**: `cargo check` to quickly validate syntax
2. **Build and test**: `cargo build && cargo test`
3. **Run the application**: `cargo run`
4. **Before committing**: `cargo fmt && cargo clippy --all-targets --all-features -- -D warnings`

### Build Artifacts and Output
- Compiled binary location: `./target/debug/ingredients` (~136MB)
- Release binary location: `./target/release/ingredients` (after `cargo build --release`)
- You can run the binary directly: `./target/debug/ingredients`
- Expected output: Bot attempts to start, fails with network error if no valid TELEGRAM_BOT_TOKEN (this is expected behavior)

## Validation

### Manual Testing Scenarios
- **ALWAYS run the complete workflow after making changes**:
  1. `cargo check` -- must pass without errors (use timeout 120+ seconds)
  2. `cargo build` -- must complete successfully (use timeout 300+ seconds)
  3. `cargo test` -- must pass all 63 tests (use timeout 120+ seconds)
  4. `cargo run` -- should attempt to start bot (network error without valid token is expected)
- **Test the compiled binary directly**: `./target/debug/ingredients` should behave identically to `cargo run`
- **Always run formatting and linting**: `cargo fmt && cargo clippy --all-targets --all-features -- -D warnings` before finishing work
- **Test OCR functionality**: Create test image and verify Tesseract works:
  ```bash
  convert -size 400x100 xc:white -font DejaVu-Sans -pointsize 20 -fill black -gravity center -annotate +0+0 "Hello from OCR test!" test_image.png
  tesseract test_image.png test_output -l eng
  cat test_output.txt  # Should output: "Hello from OCR test!"
  rm test_image.png test_output.txt
  ```

### Expected Command Output
- `cargo run` output: Bot starts initialization, then network error if no valid token (expected)
- `cargo test` output: "running 63 tests" with "test result: ok. 63 passed; 0 failed"
- `cargo build` output: "Finished \`dev\` profile [unoptimized + debuginfo] target(s)"
- `cargo clippy` output: Should complete with no warnings when using `-D warnings` flag

### System Validation
- **SQLite**: Test with `sqlite3 test.db "SELECT 'Database connection works';" && rm test.db`
- **Tesseract**: Test with `tesseract --version` (should show version 5.3.4+ with leptonica)
- **Required languages**: English (eng) and French (fra) OCR support included

## Common Tasks

### Repository Structure
```
.
├── .env.example               # Environment variables template
├── .git/
├── .github/
│   └── copilot-instructions.md
├── .gitignore
├── Cargo.lock               # Dependency lock file (committed)
├── Cargo.toml               # Project configuration
├── data_model.md            # Database schema documentation
├── locales/                 # Internationalization files
│   ├── en/main.ftl         # English localization
│   └── fr/main.ftl         # French localization
├── src/                    # Rust source code
│   ├── bot.rs              # Telegram bot logic and message handlers
│   ├── db.rs               # Database operations and schema
│   ├── localization.rs     # Internationalization support
│   ├── main.rs             # Application entry point
│   └── ocr.rs              # OCR processing with Tesseract
└── target/                 # Build artifacts (excluded from git)
```

### Cargo.toml Key Dependencies
```toml
[package]
name = "ingredients"
version = "0.1.0"
edition = "2021"

[dependencies]
teloxide = "0.17.0"          # Telegram bot framework
rusqlite = "0.37.0"          # SQLite database
leptess = "0.14"             # Tesseract OCR bindings
image = "0.24"               # Image processing
fluent = "0.16"              # Internationalization
```

### Application Features
- **Telegram Bot Commands**:
  - `/start` - Welcome message with feature overview
  - `/help` - Detailed usage instructions
- **Image Processing**: 
  - Supports PNG, JPG, JPEG, BMP, TIFF, TIF formats
  - File size limits: 10MB for JPEG, 5MB for others
  - Processes images with Tesseract OCR
- **Database**: SQLite with full-text search (FTS5) for extracted text
- **Localization**: English and French support via Fluent files
- **Error Handling**: Circuit breaker pattern for OCR reliability

### Adding New Dependencies
- Add dependencies to `Cargo.toml` under `[dependencies]` section
- Run `cargo build` to download and compile new dependencies (NEVER CANCEL: use timeout 300+ seconds)
- Use `cargo tree` to view dependency graph

### Adding Tests
- Add `#[cfg(test)]` module to source files for unit tests
- Current codebase has 63 tests across all modules
- Run `cargo test` to execute all tests (NEVER CANCEL: use timeout 120+ seconds)

### Best Practices
- Always run `cargo check` first for quick feedback (NEVER CANCEL: use timeout 120+ seconds)
- Use `cargo fmt` to maintain consistent code formatting
- Run `cargo clippy --all-targets --all-features -- -D warnings` to catch common mistakes and ensure clean code
- Build artifacts are in `target/` directory (excluded from git via .gitignore)
- Copy `.env.example` to `.env` and configure environment variables for testing
- Test OCR functionality with sample images to ensure system dependencies work correctly