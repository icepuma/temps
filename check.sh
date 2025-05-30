#!/usr/bin/env bash
set -euo pipefail

# Format all code including examples
cargo fmt --all

# Run clippy on workspace including examples
cargo clippy --workspace --tests --examples --all-features --all-targets

# Run all tests
cargo nextest run --workspace --all-features

# Run examples to ensure they compile and execute properly
echo "Running chrono example..."
cargo run --example chrono_example --features chrono

echo -e "\nRunning jiff example..."
cargo run --example jiff_example --features jiff
