#!/bin/bash
# Clean all build artifacts
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

# Clean snapcraft artifacts
rm -rf overlay/ parts/ prime/ stage/

# Clean output directory
rm -rf dist/

# Clean Rust build
cargo clean 2>/dev/null || true

echo "All build artifacts cleaned."
