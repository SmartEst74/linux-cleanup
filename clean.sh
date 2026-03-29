#!/bin/bash
# Clean snap build artifacts
rm -rf overlay/ parts/ prime/ stage/ *.snap

# Clean Rust build artifacts  
cargo clean
