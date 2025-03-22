#!/bin/bash

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
function log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

function log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

function log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

function log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if command exists
function command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Check if directory exists
function dir_exists() {
    [ -d "$1" ]
}

# Check if file exists
function file_exists() {
    [ -f "$1" ]
}

# Create directory if it doesn't exist
function ensure_dir() {
    if ! dir_exists "$1"; then
        mkdir -p "$1"
        log_info "Created directory: $1"
    fi
}

# Clean up temporary files
function cleanup() {
    local temp_dir="$1"
    if dir_exists "$temp_dir"; then
        rm -rf "$temp_dir"
        log_info "Cleaned up temporary directory: $temp_dir"
    fi
}

# Measure execution time
function measure_time() {
    local start_time=$(date +%s.%N)
    "$@"
    local end_time=$(date +%s.%N)
    local duration=$(echo "$end_time - $start_time" | bc)
    echo "$duration"
} 