default := check

# Run the full check suite (fmt + clippy + tests)
check:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace

# Auto-format all code
fmt:
    cargo fmt --all

# Run clippy lints
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# Run all tests
test:
    cargo test --workspace

# Build release binary
build:
    cargo build --release

# Build documentation
doc:
    cargo doc --workspace --no-deps

# Build documentation and open in browser
doc-open:
    cargo doc --workspace --no-deps --open

# Clean build artifacts
clean:
    cargo clean
