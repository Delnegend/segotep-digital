#!/bin/bash
set -e

# Install Rust toolchain via rustup if not already present
if ! command -v rustup &> /dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
fi

export PATH="$HOME/.cargo/bin:$PATH"

# Ensure components are installed
rustup component add rustfmt clippy

# Pre-fetch cargo dependencies
cargo fetch || true
