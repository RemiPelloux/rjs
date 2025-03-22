#!/bin/bash

# Exit on error
set -e

# Source common functions
source "$(dirname "$0")/../utils/common.sh"

# Function to check dependencies
function check_dependencies() {
    if ! command_exists cargo; then
        log_error "Cargo is not installed"
        exit 1
    fi
}

# Function to format code
function format_code() {
    log_info "Formatting code..."
    cargo fmt --all
}

# Function to check code
function check_code() {
    log_info "Checking code..."
    cargo check
}

# Function to build project
function build_project() {
    log_info "Building project..."
    cargo build --release
}

# Function to run linter
function run_linter() {
    log_info "Running linter..."
    cargo clippy -- -D warnings
}

# Main execution
function main() {
    check_dependencies
    
    # Format and check code
    format_code
    check_code
    
    # Run linter
    run_linter
    
    # Build project
    build_project
    
    log_success "Build completed successfully"
}

# Execute main function
main 