#!/bin/bash

# Exit on error
set -e

# Source common functions
source "$(dirname "$0")/../utils/common.sh"

# Function to check if there are changes to commit
function check_changes() {
    if git diff-index --quiet HEAD --; then
        log_error "No changes to commit"
        exit 1
    fi
}

# Function to get commit message
function get_commit_message() {
    local message="$1"
    if [ -z "$message" ]; then
        read -p "Enter commit message: " message
        if [ -z "$message" ]; then
            log_error "Commit message cannot be empty"
            exit 1
        fi
    fi
    echo "$message"
}

# Function to setup git repository
function setup_git() {
    if [ ! -d .git ]; then
        log_info "Initializing git repository..."
        git init
        create_gitignore
    fi
}

# Function to create .gitignore
function create_gitignore() {
    log_info "Creating .gitignore..."
    cat > .gitignore << EOL
/target/
**/*.rs.bk
Cargo.lock
.DS_Store
.env
*.log
node_modules/
dist/
build/
coverage/
.idea/
.vscode/
*.swp
*.swo
EOL
}

# Function to check and setup remote
function setup_remote() {
    if ! git remote | grep -q "^origin$"; then
        read -p "Enter remote repository URL: " remote_url
        if [ -z "$remote_url" ]; then
            log_error "Remote URL cannot be empty"
            exit 1
        fi
        git remote add origin "$remote_url"
    fi
}

# Main execution
function main() {
    check_changes
    local commit_message=$(get_commit_message "$1")
    setup_git
    git add .
    git commit -m "$commit_message"
    setup_remote
    git push -u origin main
}

# Execute main function
main "$1" 