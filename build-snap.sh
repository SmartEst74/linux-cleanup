#!/bin/bash
set -e

# Build the snap package professionally
# Usage: ./build-snap.sh [--no-cleanup]

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BUILD_DIR="/tmp/linux-cleanup-build-$$"
OUTPUT_DIR="$SCRIPT_DIR/dist"

echo "=== Linux Cleanup Snap Builder ==="
echo "Build directory: $BUILD_DIR"
echo "Output directory: $OUTPUT_DIR"
echo ""

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Run snapcraft with build output to temp directory
cd "$SCRIPT_DIR"
snapcraft --destructive-mode 2>&1

# Move the snap to dist folder
if ls linux-cleanup_*.snap 1> /dev/null 2>&1; then
    mv linux-cleanup_*.snap "$OUTPUT_DIR/"
    echo ""
    echo "=== Build Complete ==="
    echo "Snap package: $OUTPUT_DIR/$(ls $OUTPUT_DIR/*.snap)"
    
    # Clean up snapcraft artifacts
    rm -rf overlay/ parts/ prime/ stage/
    echo "Build artifacts cleaned."
else
    echo "Error: Snap file not found after build"
    exit 1
fi
