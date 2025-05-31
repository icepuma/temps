# Default recipe runs check
default: check

# Format all code including examples
format:
    cargo fmt --all

# Run clippy on workspace including examples
lint:
    cargo clippy --workspace --tests --examples --all-features --all-targets

# Run all tests with nextest
test:
    cargo nextest run --workspace --all-features

# Run chrono example
example-chrono:
    @echo "Running chrono example..."
    cargo run --example chrono_example --features chrono

# Run jiff example
example-jiff:
    @echo -e "\nRunning jiff example..."
    cargo run --example jiff_example --features jiff

# Run all examples
examples: example-chrono example-jiff

# Run complete check (format, lint, test, examples)
check: format lint test examples