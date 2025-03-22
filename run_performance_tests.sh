#!/bin/bash
# Script to run performance tests and save results

set -e  # Exit on error

echo "Building RJS in release mode for accurate performance testing..."
cargo build --release

echo "Running performance tests..."
RUST_BACKTRACE=1 cargo test --test performance -- --nocapture > performance_results.txt 2>&1

if [ $? -eq 0 ]; then
    echo "✅ Performance tests completed successfully!"
    echo "Results have been saved to performance_results.txt"
    
    # Extract and display the summary
    echo -e "\nPerformance Summary:"
    grep -A 10 "=== Performance Summary ===" performance_results.txt
else
    echo "❌ Performance tests failed. Check performance_results.txt for details."
    exit 1
fi 