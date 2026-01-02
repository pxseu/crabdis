#!/usr/bin/env bash

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

# Check if version is provided
if [ -z "$1" ]; then
    print_error "Usage: $0 <version>"
    echo "Example: $0 0.1.34"
    exit 1
fi

VERSION="$1"

# Validate version format (semantic versioning)
if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    print_error "Invalid version format. Expected: X.Y.Z (e.g., 0.1.34)"
    exit 1
fi

# Check if we're in the project root
if [ ! -f "Cargo.toml" ]; then
    print_error "Cargo.toml not found. Please run this script from the project root."
    exit 1
fi

# Check if git working directory is clean
if [ -n "$(git status --porcelain)" ]; then
    print_warning "Git working directory is not clean:"
    git status --short
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        print_info "Aborted."
        exit 1
    fi
fi

print_info "Updating version to $VERSION..."

# Update [workspace.package] version using awk to be more precise
awk -v version="$VERSION" '
    /^\[workspace\.package\]/ { in_workspace_package=1 }
    /^\[/ && !/^\[workspace\.package\]/ { in_workspace_package=0 }
    in_workspace_package && /^version = / { print "version = \"" version "\""; next }
    { print }
' Cargo.toml > Cargo.toml.tmp && mv Cargo.toml.tmp Cargo.toml

# Update crabdis-core dependency version
sed -i.bak 's/^crabdis-core = { path = "crabdis-core", version = ".*" }$/crabdis-core = { path = "crabdis-core", version = "'"$VERSION"'" }/' Cargo.toml

# Remove backup files
rm -f Cargo.toml.bak Cargo.toml.tmp

print_info "Running cargo check to update Cargo.lock..."
cargo check --quiet

print_info "Staging changes..."
git add Cargo.toml Cargo.lock

print_info "Creating commit with message: $VERSION"
git commit -m "$VERSION"

print_info "Creating annotated signed tag: v$VERSION"
git tag -asm "v$VERSION" "v$VERSION"

print_info "✓ Release $VERSION prepared successfully!"
echo ""
print_info "Next steps:"
echo "  1. Review changes: git show"
echo "  2. Push changes: git push && git push --tags"
echo "  3. Publish to crates.io: cargo publish"
echo ""
print_warning "To undo this release (before pushing):"
echo "  git tag -d v$VERSION"
echo "  git reset --hard HEAD~1"
