#!/bin/bash
set -e

# Test script for rNGA workspace

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

print_error() {
    echo -e "${RED}Error:${NC} $1"
}

# Parse arguments
PACKAGE=""
VERBOSE=false
COVERAGE=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --lib)
            PACKAGE="rnga"
            shift
            ;;
        --cli)
            PACKAGE="rnga-cli"
            shift
            ;;
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        --coverage)
            COVERAGE=true
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --lib            Test only the rnga library"
            echo "  --cli            Test only the rnga-cli"
            echo "  -v, --verbose    Run tests with verbose output"
            echo "  --coverage       Generate test coverage (requires cargo-llvm-cov)"
            echo "  -h, --help       Show this help message"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Build test command
CMD="cargo test"

if [ -n "$PACKAGE" ]; then
    CMD="$CMD -p $PACKAGE"
else
    CMD="$CMD --workspace"
fi

if [ "$VERBOSE" = true ]; then
    CMD="$CMD -- --nocapture"
fi

# Run tests
if [ "$COVERAGE" = true ]; then
    if ! command -v cargo-llvm-cov &> /dev/null; then
        print_error "cargo-llvm-cov is not installed."
        echo "Install with: cargo install cargo-llvm-cov"
        exit 1
    fi
    print_status "Running tests with coverage..."
    cargo llvm-cov --workspace --html
    print_status "Coverage report generated at target/llvm-cov/html/index.html"
else
    print_status "Running tests..."
    echo "Command: $CMD"
    echo ""
    $CMD
fi

echo ""
print_status "All tests passed!"

