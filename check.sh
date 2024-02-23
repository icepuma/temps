#!/usr/bin/env bash
set -euo pipefail

cargo fmt --all
cargo clippy --tests --all-features --all-targets
cargo test --tests --all-features --all-targets
