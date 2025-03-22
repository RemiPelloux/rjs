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
    
    # Run basic tests from project directory
    log_info "Running basic command tests"
    cd "$PROJECT_DIR"
    cargo test --test functional -- --nocapture
    
    # Test init command with --yes flag
    log_info "Testing init command with --yes flag"
    local test_dir="$TEMP_DIR/init_test"
    ensure_dir "$test_dir"
    cd "$test_dir"
    
    # Run the init command with --yes flag
    "$PROJECT_DIR/target/debug/rjs" init --yes
    
    # Verify package.json was created
    if [ -f "package.json" ]; then
        log_success "Init command with --yes flag succeeded"
    else
        log_error "Init command with --yes flag failed: package.json not created"
        exit 1
    fi
    
    # Return to project directory
    cd "$PROJECT_DIR"
}

# Function to run performance tests
function run_performance_tests() {
    log_info "Running performance tests..."
    
    # Ensure we're in the project directory
    cd "$PROJECT_DIR"
    
    # Ensure the release binary is built
    cargo build --release
    
    # Export the project directory for the performance tests
    export RJS_PROJECT_DIR="$PROJECT_DIR"
    cargo test --test performance -- --nocapture
}

# Function to setup test environment
function setup_test_env() {
    # Ensure we're in the project directory
    cd "$PROJECT_DIR"
    
    # Build the project first
    log_info "Building project..."
    cargo build
    
    # Create temp directory
    ensure_dir "$TEMP_DIR"
    
    # Export environment variables
    export RJS_TEST_TEMP_DIR="$TEMP_DIR"
    log_info "Test temp directory: $RJS_TEST_TEMP_DIR"
}

# Function to cleanup test environment
function cleanup_test_env() {
    cleanup "$TEMP_DIR"
    unset RJS_TEST_TEMP_DIR
    unset RJS_PROJECT_DIR
    
    # Return to project directory
    cd "$PROJECT_DIR"
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