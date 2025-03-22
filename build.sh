#!/bin/bash
# Build script for RJS

set -e  # Exit on error

echo "Building RJS project..."
cargo build

if [ $? -eq 0 ]; then
    echo "✅ Build successful!"
    
    echo "Running tests..."
    cargo test
    
    echo "Checking formatting..."
    if command -v cargo-fmt &> /dev/null; then
        cargo fmt --check
    else
        echo "⚠️ cargo-fmt not found, skipping format check. Install it with 'rustup component add rustfmt'."
    fi
    
    echo "Checking for linting issues..."
    if command -v cargo-clippy &> /dev/null; then
        cargo clippy -- -D warnings
    else
        echo "⚠️ clippy not found, skipping linting. Install it with 'rustup component add clippy'."
    fi
    
    echo "Built executable: target/debug/rjs"
    echo "Run with: ./target/debug/rjs [command]"
    echo "Example: ./target/debug/rjs init"
else
    echo "❌ Build failed."
    exit 1
fi 