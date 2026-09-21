# Default recipe runs when `just` is invoked without arguments
default:
    @just --list

# Run verification suite (formatting, clippy, tests)
check: fmt-check lint test

# Check code formatting
fmt-check:
    cargo fmt --check

# Format code
fmt:
    cargo fmt

# Run Clippy lints with warnings as errors
lint:
    cargo clippy --all-targets --all-features -- -D warnings

# Run all tests
test:
    cargo test --all-targets --all-features
