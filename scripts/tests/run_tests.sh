#!/bin/bash

# Exit on error
set -e

# Source common functions
source "$(dirname "$0")/../utils/common.sh"

# Test directories
PROJECT_DIR="$(realpath "$(dirname "$0")/../..")"
TEST_DIR="$PROJECT_DIR/tests"
TEMP_DIR="/tmp/rjs_test_$(date +%s)"

# Function to run functional tests
function run_functional_tests() {
    log_info "Running functional tests..."
    cargo test --test functional -- --nocapture
}

# Function to run performance tests
function run_performance_tests() {
    log_info "Running performance tests..."
    # Ensure the release binary is built
    cargo build --release
    # Export the project directory for the performance tests
    export RJS_PROJECT_DIR="$PROJECT_DIR"
    cargo test --test performance -- --nocapture
}

# Function to setup test environment
function setup_test_env() {
    ensure_dir "$TEMP_DIR"
    # Stay in project directory
    export RJS_TEST_TEMP_DIR="$TEMP_DIR"
    log_info "Test temp directory: $RJS_TEST_TEMP_DIR"
}

# Function to cleanup test environment
function cleanup_test_env() {
    cleanup "$TEMP_DIR"
    unset RJS_TEST_TEMP_DIR
    unset RJS_PROJECT_DIR
}

# Function to check test dependencies
function check_dependencies() {
    if ! command_exists cargo; then
        log_error "Cargo is not installed"
        exit 1
    fi
}

# Main execution
function main() {
    check_dependencies
    setup_test_env
    
    # Run tests
    run_functional_tests
    run_performance_tests
    
    cleanup_test_env
    log_success "All tests completed successfully"
}

# Execute main function
main 