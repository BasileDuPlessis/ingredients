# Ingredients Rust Application

Ingredients is a Telegram bot application built in Rust that receives pictures of ingredient lists for recipes and comments from users, extracts text from images using OCR (Optical Character Recognition), and stores all the extracted text and comments in a full-text searchable database table. Users can later query the stored content through the Telegram interface.

Always reference these instructions first and fallback to search or bash commands only when you encounter unexpected information that does not match the info here.

## Working Effectively

### Prerequisites - CRITICAL: System Dependencies Required
- Rust toolchain is already installed (rustc 1.89.0, cargo 1.89.0)
- **REQUIRED system packages for OCR functionality**:
  ```bash
  sudo apt-get update
  sudo apt-get install -y libleptonica-dev libtesseract-dev pkg-config tesseract-ocr-eng tesseract-ocr-fra tesseract-ocr
  ```
- **NEVER try to build without these system dependencies** - the build will fail with Leptonica errors

### Essential Development Commands - NEVER CANCEL BUILDS
- **Check code without building**: `cargo check` -- takes ~45 seconds for first run, ~0.2 seconds for incremental
- **Build the project**: `cargo build` -- takes ~55 seconds for first build, ~0.2 seconds for incremental builds. **NEVER CANCEL - Set timeout to 180+ seconds**
- **Build for release**: `cargo build --release` -- takes ~2 minutes. **NEVER CANCEL - Set timeout to 300+ seconds**
- **Run tests**: `cargo test` -- takes ~0.5 seconds (currently 63 tests, 62+ should pass, occasional flaky language detection test)
- **Run the application**: `cargo run` -- takes ~0.2 seconds to run after build (will fail without valid TELEGRAM_BOT_TOKEN)
- **Format code**: `cargo fmt` -- takes ~0.4 seconds
- **Lint code**: `cargo clippy` -- takes ~3 seconds for analysis
- **Generate documentation**: `cargo doc --no-deps` -- takes ~4 seconds
- **Clean build artifacts**: `cargo clean` -- removes target/ directory

### Development Workflow
1. **ALWAYS start with system dependencies**: Install OCR libraries if not present
2. **Check syntax first**: `cargo check` to quickly validate syntax
3. **Build and test**: `cargo build && cargo test` (NEVER CANCEL these commands)
4. **Run the application**: `cargo run` (requires .env file with valid tokens)
5. **Before committing**: `cargo fmt && cargo clippy`

### Build Artifacts and Output
- Compiled binary location: `./target/debug/ingredients` (~136MB file)
- Release binary location: `./target/release/ingredients` (~9.7MB file after `cargo build --release`)
- You can run the binary directly: `./target/debug/ingredients`
- Expected output on success: Bot starts, initializes database, begins polling for Telegram messages
- Expected output on failure: Network error with invalid bot token (this is normal without proper configuration)

## Validation

### Manual Testing Scenarios
- **ALWAYS run the complete workflow after making changes**:
  1. `cargo check` -- must pass without errors (45s first time, 0.2s incremental)
  2. `cargo build` -- must complete successfully (55s first time, 0.2s incremental) 
  3. `cargo test` -- must pass all 63 tests (0.5s)
  4. `cargo run` -- must attempt to start the bot (will fail with network error without valid token)
- **Test the compiled binary directly**: `./target/debug/ingredients` should behave identically to `cargo run`
- **Always run formatting and linting**: `cargo fmt && cargo clippy` before finishing work
- **Environment validation**: Copy `.env.example` to `.env` and verify bot tries to start (expect network failure)

### Expected Command Output
- `cargo test` output: "running 63 tests" with "test result: ok. 62 passed; 1 failed" (occasional flaky test acceptable)
- `cargo build` output: "Finished \`dev\` profile [unoptimized + debuginfo] target(s) in XX.XXs"
- `cargo run` output: Bot initialization, database setup, then network error (expected without valid token)
- Application with valid token: "Starting Ingredients Telegram Bot", "Bot initialized", no crash

## Common Tasks

### Repository Structure
```
.
├── .git/
├── .github/
│   └── copilot-instructions.md
├── .gitignore
├── .env.example                    -- Copy to .env and configure
├── Cargo.toml                      -- 17 major dependencies
├── Cargo.lock                      -- Dependency lock file
├── data_model.md                   -- Database schema documentation
├── locales/                        -- Internationalization
│   ├── en/                         -- English messages
│   └── fr/                         -- French messages
├── src/
│   ├── main.rs                     -- Application entry point
│   ├── bot.rs                      -- Telegram bot logic
│   ├── db.rs                       -- Database operations
│   ├── localization.rs             -- i18n support
│   └── ocr.rs                      -- OCR processing logic
└── target/ (created after build)   -- Build artifacts
```

### Cargo.toml Contents
```toml
[package]
name = "ingredients"
version = "0.1.0"
edition = "2021"

[dependencies]
teloxide = "0.17.0"               -- Telegram bot framework
rusqlite = "0.37.0"               -- SQLite database
tokio = { version = "1.47.1", features = ["full"] }
leptess = "0.14.0"                -- Tesseract OCR bindings
image = "0.24"                    -- Image processing
fluent = "0.16"                   -- Internationalization
# ... 11 more dependencies
```

### Source Code Location and Key Components
- **Main application entry point**: `src/main.rs` -- Sets up bot, database, and dispatcher
- **Bot message handling**: `src/bot.rs` -- Processes Telegram messages, handles OCR workflow
- **Database operations**: `src/db.rs` -- SQLite with FTS for searchable text storage
- **OCR processing**: `src/ocr.rs` -- Tesseract integration with error handling and retries
- **Internationalization**: `src/localization.rs` + `locales/` -- English and French support

### Environment Setup - CRITICAL
- **Always copy environment template**: `cp .env.example .env`
- **Required environment variables**:
  - `TELEGRAM_BOT_TOKEN=your_bot_token_here` (required for operation)
  - `DATABASE_URL=ingredients.db` (SQLite database path)
  - Optional: `HEALTH_PORT=8080`, `GRAFANA_ADMIN_PASSWORD`
- **Database initialization**: Automatic on first run, creates `ingredients.db`

### Adding New Dependencies
- Add dependencies to `Cargo.toml` under `[dependencies]` section
- Run `cargo build` to download and compile new dependencies (allow 60+ seconds for first build)
- Use `cargo tree --depth 1` to view dependency graph

### Adding Tests
- Tests are in `#[cfg(test)]` modules within source files (`src/bot.rs`, `src/db.rs`, `src/ocr.rs`)
- 62+ tests should pass after installing system dependencies (1 language detection test may be flaky)
- No separate `tests/` directory for integration tests
- Run `cargo test` to execute all tests (0.5 seconds)

### Manual Testing After Changes
- **Database functionality**: Tests cover CRUD operations and FTS search
- **OCR functionality**: Tests cover image processing, format validation, error handling
- **Bot functionality**: Tests cover message handling, command responses, localization
- **Integration test**: Try `cargo run` -- should initialize database and attempt Telegram connection

### Best Practices
- **Always install system dependencies first** or builds will fail
- **Always run `cargo check` first** for quick feedback (45s first time, then 0.2s)
- **Use `cargo fmt`** to maintain consistent code formatting (0.4s)
- **Run `cargo clippy`** to catch common mistakes and improve code quality (3s)
- **Build artifacts are in `target/` directory** (excluded from git via .gitignore)
- **Set long timeouts for builds** (180s+ for debug, 300s+ for release)
- **Environment file is required** for application to run properly