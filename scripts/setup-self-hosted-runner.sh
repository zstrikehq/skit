#!/bin/bash
# Setup script for self-hosted GitHub Actions runner
# Run this once on your self-hosted runner to install required dependencies

set -e

echo "Setting up self-hosted runner for skit builds..."

# Update package list
echo "Updating package list..."
sudo apt-get update

# Install essential build tools
echo "Installing build-essential..."
sudo apt-get install -y build-essential

# Install cross-compilation tools for ARM64
echo "Installing ARM64 cross-compilation tools..."
sudo apt-get install -y gcc-aarch64-linux-gnu

# Install Rust if not present
if ! command -v rustup &> /dev/null; then
    echo "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

# Add required Rust targets
echo "Adding Rust targets..."
rustup target add x86_64-unknown-linux-gnu
rustup target add aarch64-unknown-linux-gnu

echo "Setup complete! Your self-hosted runner is ready for skit builds."
echo ""
echo "Installed tools:"
echo "  - gcc (native compilation)"
echo "  - aarch64-linux-gnu-gcc (ARM64 cross-compilation)"
echo "  - Rust toolchain with x86_64 and ARM64 targets"