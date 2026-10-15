#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
cargo fmt --all --check
cargo test --all-targets --locked "$@"
cargo test --doc --locked "$@"
cargo clippy --all-targets --locked "$@" -- -D warnings
