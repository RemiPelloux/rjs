#!/bin/bash

# Exit on error
set -e

# Source common functions
source "$(dirname "$0")/../utils/common.sh"

# Test directories
PROJECT_DIR="$(realpath "$(dirname "$0")/../..")"
TEST_DIR="$PROJECT_DIR/tests"
TEMP_DIR="/tmp/rjs_perf_test_$(date +%s)"
BINARY_PATH="$PROJECT_DIR/target/release/rjs"

# Function to compile the project in release mode
function precompile_project() {
    log_info "Precompiling project in release mode..."
    cd "$PROJECT_DIR"
    cargo build --release
    
    # Verify binary exists
    if [ ! -f "$BINARY_PATH" ]; then
        log_error "Failed to build release binary at $BINARY_PATH"
        exit 1
    fi
    
    log_success "Project precompiled successfully"
}

# Function to measure a specific command execution time
function benchmark_command() {
    local cmd="$1"
    local description="$2"
    local iterations=${3:-3}
    local total_time=0
    
    log_info "Benchmarking: $description"
    
    # Run the command multiple times to get an average
    for i in $(seq 1 $iterations); do
        log_info "  Iteration $i of $iterations"
        
        # Create a clean test directory for each iteration
        local test_dir="$TEMP_DIR/benchmark_${i}"
        ensure_dir "$test_dir"
        cd "$test_dir"
        
        # Measure execution time
        local start_time=$(date +%s.%N)
        eval "$cmd"
        local end_time=$(date +%s.%N)
        local duration=$(echo "$end_time - $start_time" | bc)
        total_time=$(echo "$total_time + $duration" | bc)
        
        log_info "  Time: ${duration}s"
        
        # Verify the command worked as expected
        if [ "$description" = "init with --yes flag" ] && [ ! -f "package.json" ]; then
            log_error "Benchmark failed: package.json not created"
            exit 1
        fi
    done
    
    # Calculate average
    local avg_time=$(echo "scale=4; $total_time / $iterations" | bc)
    log_success "Average execution time for '$description': ${avg_time}s"
}

# Function to run performance tests with timing
function run_performance_tests() {
    log_info "Running performance tests..."
    
    # Precompile for better performance
    precompile_project
    
    # Benchmark specific commands
    benchmark_command "'$BINARY_PATH' init --yes" "init with --yes flag"
    
    # Return to project directory
    cd "$PROJECT_DIR"
    
    # Run the formal performance tests
    log_info "Running formal performance tests..."
    export RJS_PROJECT_DIR="$PROJECT_DIR"
    local duration=$(measure_time cargo test --release --test performance -- --nocapture)
    log_success "Performance tests completed in ${duration}s"
}

# Function to setup test environment
function setup_test_env() {
    # Ensure we're in the project directory
    cd "$PROJECT_DIR"
    
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
    
    if ! command_exists bc; then
        log_error "bc is not installed (required for calculations)"
        exit 1
    fi
}

# Main execution
function main() {
    log_info "Running comprehensive performance test suite with environment setup"
    log_info "Note: For quick performance test runs during development, use 'cargo test --release --test performance' directly"
    
    check_dependencies
    setup_test_env
    
    # Run performance tests
    run_performance_tests
    
    cleanup_test_env
    log_success "Performance tests completed successfully"
}

# Execute main function
main 