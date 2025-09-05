# Ingredients Rust Application

Ingredients is a simple Rust command-line application that currently outputs "Hello, world!" when executed. This is a basic Rust project managed with Cargo.

Always reference these instructions first and fallback to search or bash commands only when you encounter unexpected information that does not match the info here.

## Working Effectively

### Prerequisites
- Rust toolchain is already installed (rustc 1.89.0, cargo 1.89.0)
- No additional dependencies or setup required

### Essential Development Commands
- **Build the project**: `cargo build` -- takes ~5 seconds for first build, ~0.2 seconds for incremental builds
- **Build for release**: `cargo build --release` -- takes ~0.3 seconds  
- **Run the application**: `cargo run` -- takes ~0.1 seconds to run after build
- **Run tests**: `cargo test` -- takes ~0.7 seconds (currently 0 tests exist)
- **Check code without building**: `cargo check` -- takes ~0.1 seconds
- **Format code**: `cargo fmt` -- instant formatting
- **Lint code**: `cargo clippy` -- takes ~1 second for linting analysis
- **Generate documentation**: `cargo doc` -- takes ~5 seconds
- **Clean build artifacts**: `cargo clean` -- removes target/ directory

### Development Workflow
1. **Always start with**: `cargo check` to quickly validate syntax
2. **Build and test**: `cargo build && cargo test`
3. **Run the application**: `cargo run`
4. **Before committing**: `cargo fmt && cargo clippy`

### Build Artifacts and Output
- Compiled binary location: `./target/debug/ingredients`
- Release binary location: `./target/release/ingredients` (after `cargo build --release`)
- You can run the binary directly: `./target/debug/ingredients`
- Expected output: "Hello, world!"

## Validation

### Manual Testing Scenarios
- **ALWAYS run the complete workflow after making changes**:
  1. `cargo check` -- must pass without errors
  2. `cargo build` -- must complete successfully  
  3. `cargo test` -- must pass (even with 0 tests)
  4. `cargo run` -- must output "Hello, world!"
- **Test the compiled binary directly**: `./target/debug/ingredients` should output "Hello, world!"
- **Always run formatting and linting**: `cargo fmt && cargo clippy` before finishing work

### Expected Command Output
- `cargo run` output: "Hello, world!"
- `cargo test` output: "running 0 tests" with "test result: ok"
- `cargo build` output: "Finished \`dev\` profile [unoptimized + debuginfo] target(s)"

## Common Tasks

### Repository Structure
```
.
├── .git/
├── .gitignore
├── Cargo.toml
├── Cargo.lock
├── src/
│   └── main.rs
└── target/ (created after build)
```

### Cargo.toml Contents
```toml
[package]
name = "ingredients"
version = "0.1.0"
edition = "2024"

[dependencies]
```

### Source Code Location
- Main application entry point: `src/main.rs`
- Current functionality: Simple "Hello, world!" program

### Adding New Dependencies
- Add dependencies to `Cargo.toml` under `[dependencies]` section
- Run `cargo build` to download and compile new dependencies
- Use `cargo tree` to view dependency graph

### Adding Tests
- Add `#[cfg(test)]` module to source files for unit tests
- Create `tests/` directory for integration tests
- Run `cargo test` to execute all tests

### Best Practices
- Always run `cargo check` first for quick feedback
- Use `cargo fmt` to maintain consistent code formatting
- Run `cargo clippy` to catch common mistakes and improve code quality
- Build artifacts are in `target/` directory (excluded from git via .gitignore)
- No special environment setup or external services required