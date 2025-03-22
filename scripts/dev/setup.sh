#!/bin/bash

# Exit on error
set -e

# Source common functions
source "$(dirname "$0")/../utils/common.sh"

# Function to check dependencies
function check_dependencies() {
    local deps=("cargo" "rustc" "git")
    for dep in "${deps[@]}"; do
        if ! command_exists "$dep"; then
            log_error "$dep is not installed"
            exit 1
        fi
    done
}

# Function to install Rust toolchain
function install_rust_toolchain() {
    log_info "Installing Rust toolchain..."
    rustup default stable
    rustup update
}

# Function to install development tools
function install_dev_tools() {
    log_info "Installing development tools..."
    cargo install cargo-edit
    cargo install cargo-watch
    cargo install cargo-expand
    cargo install cargo-audit
}

# Function to setup git hooks
function setup_git_hooks() {
    log_info "Setting up git hooks..."
    local hooks_dir=".git/hooks"
    ensure_dir "$hooks_dir"
    
    # Pre-commit hook
    cat > "$hooks_dir/pre-commit" << 'EOL'
#!/bin/bash
set -e
cargo fmt --all -- --check
cargo clippy -- -D warnings
cargo test
EOL
    chmod +x "$hooks_dir/pre-commit"
}

# Function to create initial project structure
function create_project_structure() {
    log_info "Creating project structure..."
    local dirs=(
        "src/cli/commands"
        "src/dependency"
        "src/package"
        "src/registry"
        "src/utils"
        "tests"
    )
    
    for dir in "${dirs[@]}"; do
        ensure_dir "$dir"
    done
}

# Main execution
function main() {
    check_dependencies
    install_rust_toolchain
    install_dev_tools
    setup_git_hooks
    create_project_structure
    
    log_success "Development environment setup completed successfully"
}

# Execute main function
main 