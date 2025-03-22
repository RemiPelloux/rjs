#!/bin/bash

# Exit on error
set -e

# Source common functions
source "$(dirname "$0")/../utils/common.sh"

# Test directories
PROJECT_DIR="$(realpath "$(dirname "$0")/../..")"
TEST_DIR="$PROJECT_DIR/tests"
TEMP_DIR="/tmp/rjs_perf_test_$(date +%s)"

# Function to run performance tests with timing
function run_performance_tests() {
    log_info "Running performance tests..."
    
    # Run each test and measure time
    local tests=(
        "test_init_command"
        "test_install_command"
        "test_list_command"
    )
    
    for test in "${tests[@]}"; do
        log_info "Running $test..."
        local duration=$(measure_time cargo test --test performance "$test" -- --nocapture)
        log_success "$test completed in ${duration}s"
    done
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
    
    # Run performance tests
    run_performance_tests
    
    cleanup_test_env
    log_success "Performance tests completed successfully"
}

# Execute main function
main 