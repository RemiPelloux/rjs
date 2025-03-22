#!/bin/bash
# Script to run all tests

set -e  # Exit on error

echo "Building RJS..."
cargo build

echo "Running functional tests..."
cargo test --test functional

echo "Running performance tests..."
./run_performance_tests.sh

echo "All tests completed successfully!" 