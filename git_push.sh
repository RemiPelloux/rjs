#!/bin/bash
# Script to handle Git operations

set -e  # Exit on error

# Check if there are changes to commit
if git diff-index --quiet HEAD --; then
    echo "No changes to commit."
    exit 0
fi

# Get commit message from user
if [ -z "$1" ]; then
    read -p "Enter commit message: " COMMIT_MESSAGE
else
    COMMIT_MESSAGE="$1"
fi

# Initialize repository if needed
if [ ! -d .git ]; then
    echo "Initializing Git repository..."
    git init
    
    # Create .gitignore if it doesn't exist
    if [ ! -f .gitignore ]; then
        echo "Creating .gitignore..."
        cat > .gitignore <<EOL
/target/
**/*.rs.bk
*.pdb
Cargo.lock
.idea/
.vscode/
*.swp
*.swo
*~
.DS_Store
node_modules/
performance_results.txt
EOL
    fi
fi

# Add all files
echo "Adding files to Git..."
git add .

# Commit changes
echo "Committing changes with message: $COMMIT_MESSAGE"
git commit -m "$COMMIT_MESSAGE"

# Configure remote if needed
if ! git remote | grep -q "origin"; then
    echo "Remote 'origin' not found. Please enter the URL for your remote repository:"
    read -p "Remote URL: " REMOTE_URL
    git remote add origin "$REMOTE_URL"
fi

# Push to origin main
echo "Pushing to origin main..."
git push -u origin main

echo "Git operations completed successfully!" 