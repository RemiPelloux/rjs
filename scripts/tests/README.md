# RJS Test Scripts

This directory contains scripts for running comprehensive tests on the RJS package manager.

## Overview

There are two main approaches to testing RJS:

1. **Direct Cargo Test Commands** (Quick Development Testing)
   ```bash
   # Run all tests in release mode
   cargo test --release
   
   # Run only functional tests
   cargo test --release --test functional
   
   # Run only performance tests
   cargo test --release --test performance
   ```

2. **Comprehensive Test Scripts** (CI/CD & Pre-Release Testing)
   ```bash
   # Run all tests with environment setup/cleanup
   ./scripts/tests/run_tests.sh
   
   # Run only performance tests with benchmarking
   ./scripts/tests/run_performance_tests.sh
   ```

## When to Use Each Approach

- Use the **Direct Cargo Test Commands** during development for quick feedback
- Use the **Comprehensive Test Scripts** for:
  - CI/CD pipelines
  - Pre-release validation
  - When you need detailed benchmarking data
  - When you need controlled test environments
  
## Script Descriptions

### run_tests.sh

This script:
- Sets up a controlled test environment
- Builds the project in release mode
- Runs functional tests
- Runs performance tests 
- Performs additional validation (like checking if `package.json` was created)
- Cleans up temporary files and directories

### run_performance_tests.sh

This script:
- Compiles the project in release mode
- Creates clean test directories for each benchmark iteration
- Runs specific commands multiple times to get accurate performance measurements
- Calculates average execution times
- Runs the formal performance tests from the codebase
- Provides detailed timing information

## Environment Variables

These scripts use the following environment variables:

- `RJS_PROJECT_DIR` - Path to the RJS project directory
- `RJS_TEST_TEMP_DIR` - Path to the temporary test directory

## Best Practices

1. Always run the comprehensive scripts before submitting a pull request
2. Use the direct commands during development
3. If you modify the test scripts, ensure they still work in both local and CI environments 