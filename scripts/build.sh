#!/bin/bash
set -e

# Build script for rNGA workspace

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

print_status() {
    echo -e "${GREEN}==>${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}Warning:${NC} $1"
}

# Parse arguments
RELEASE=false
CLEAN=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --release|-r)
            RELEASE=true
            shift
            ;;
        --clean|-c)
            CLEAN=true
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  -r, --release    Build in release mode"
            echo "  -c, --clean      Clean before building"
            echo "  -h, --help       Show this help message"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Clean if requested
if [ "$CLEAN" = true ]; then
    print_status "Cleaning build artifacts..."
    cargo clean
fi

# Build
if [ "$RELEASE" = true ]; then
    print_status "Building workspace (release)..."
    cargo build --release --workspace
    echo ""
    print_status "Release binaries:"
    ls -lh target/release/rnga 2>/dev/null || true
else
    print_status "Building workspace (debug)..."
    cargo build --workspace
    echo ""
    print_status "Debug binaries:"
    ls -lh target/debug/rnga 2>/dev/null || true
fi

echo ""
print_status "Build completed successfully!"

