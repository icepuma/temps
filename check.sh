#!/usr/bin/env bash
set -euo pipefail

cargo fmt --all
cargo clippy --workspace --tests --all-features --all-targets
cargo nextest run --workspace --all-features
