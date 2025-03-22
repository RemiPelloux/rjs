#!/bin/bash
# Setup script for RJS - Rust JavaScript Package Manager

echo "Setting up RJS development environment..."

# Check if Rust is installed
if ! command -v rustc &> /dev/null; then
    echo "Rust is not installed. Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
else
    echo "Rust is already installed. Updating..."
    rustup update
fi

# Check if needed packages are available
echo "Checking and installing required system packages..."
if command -v apt-get &> /dev/null; then
    # Debian/Ubuntu
    sudo apt-get update
    sudo apt-get install -y build-essential pkg-config libssl-dev
elif command -v brew &> /dev/null; then
    # macOS with Homebrew
    brew update
    brew install openssl pkg-config
elif command -v dnf &> /dev/null; then
    # Fedora
    sudo dnf install -y openssl-devel pkg-config gcc
elif command -v pacman &> /dev/null; then
    # Arch Linux
    sudo pacman -Sy --noconfirm openssl pkg-config
else
    echo "Unsupported package manager. Please manually install required dependencies:"
    echo "- OpenSSL development libraries"
    echo "- pkg-config"
    echo "- C compiler toolchain"
fi

# Set up directory structure
echo "Creating directory structure..."
mkdir -p src/cli/commands
mkdir -p src/dependency
mkdir -p src/registry
mkdir -p src/utils

# Build the project
echo "Building the project..."
cargo build

if [ $? -eq 0 ]; then
    echo "✅ Setup completed successfully!"
    echo "To run RJS, use: cargo run -- [command]"
    echo "Example: cargo run -- init"
else
    echo "❌ Setup failed. Please check the error messages above."
fi 