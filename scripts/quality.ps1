$ErrorActionPreference = "Stop"

cargo fmt --check
cargo test
cargo clippy --all-targets --all-features
