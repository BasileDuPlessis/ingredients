# Automated Code Review Process

This repository includes comprehensive automated code review that runs on every commit and pull request.

## Overview

The automated code review system performs the following checks:

### Basic Code Review (`.github/workflows/code-review.yml`)
- **Code formatting**: Ensures consistent code style using `rustfmt`
- **Linting**: Catches common mistakes and enforces best practices using `clippy`
- **Compilation**: Verifies code compiles without errors
- **Testing**: Runs all unit tests to ensure functionality
- **Build**: Performs release build to check optimization compatibility
- **Documentation**: Verifies documentation can be generated
- **Security audit**: Checks for known security vulnerabilities

### Advanced Quality Checks (`.github/workflows/quality-checks.yml`)
- **Test coverage**: Measures code coverage using `cargo-tarpaulin`
- **Dependency analysis**: Checks for outdated dependencies
- **License compliance**: Validates dependency licenses
- **Security scanning**: Advanced security checks using `cargo-deny`

## Triggered Events

The code review workflows are triggered on:
- **Push** to `main` and `develop` branches
- **Pull requests** targeting `main` and `develop` branches

## Local Development

### Pre-commit Hook

To run the same checks locally before committing, you can use the provided pre-commit hook:

```bash
# Copy the hook to your local git hooks directory
cp .github/pre-commit-hook.sh .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit
```

### Manual Checks

Run individual checks manually:

```bash
# Check formatting
cargo fmt --all -- --check

# Run linting
cargo clippy --all-targets --all-features -- -D warnings

# Run tests
cargo test --all-features

# Build project
cargo build --release

# Generate documentation
cargo doc --no-deps --all-features

# Security audit
cargo audit
```

## Configuration Files

- `deny.toml`: Configuration for license and security checks
- `.github/workflows/code-review.yml`: Basic code review workflow
- `.github/workflows/quality-checks.yml`: Advanced quality checks
- `.github/pre-commit-hook.sh`: Local pre-commit hook script

## Quality Standards

The code review process enforces:
- ✅ Zero compilation warnings
- ✅ All tests pass
- ✅ Consistent code formatting
- ✅ Clippy linting without warnings
- ✅ Documentation completeness
- ✅ License compatibility
- ✅ Security vulnerability checks

## Troubleshooting

If code review fails:

1. **Formatting issues**: Run `cargo fmt`
2. **Linting warnings**: Run `cargo clippy` and fix reported issues
3. **Test failures**: Run `cargo test` and fix failing tests
4. **Build errors**: Run `cargo build` and resolve compilation errors
5. **Documentation issues**: Run `cargo doc` and fix documentation errors

## Dependencies

The workflows automatically install required system dependencies:
- `libleptonica-dev` - For OCR functionality
- `libtesseract-dev` - For OCR functionality  
- `pkg-config` - For build configuration
- `tesseract-ocr-eng` - English OCR language pack
- `tesseract-ocr-fra` - French OCR language pack
- `tesseract-ocr` - OCR engine

These are required for the Ingredients bot's OCR capabilities and are automatically installed in the CI environment.