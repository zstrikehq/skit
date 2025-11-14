# SKIT (Security Kit) justfile

# Default recipe
default:
    @just --list

# Build for current platform
build:
    cargo build --release

# Build for all platforms
build-all: build-linux build-windows build-mac

# Build for Linux (x86_64) - native build
build-linux:
    cargo build --release
    mkdir -p dist/linux
    cp target/release/skit dist/linux/skit

# Build for Linux (ARM64)
build-linux-arm64:
    cargo build --release --target aarch64-unknown-linux-gnu
    mkdir -p dist/linux-arm64
    cp target/aarch64-unknown-linux-gnu/release/skit dist/linux-arm64/skit

# Build for Windows (x86_64) - Requires Windows SDK
build-windows:
    @echo "Windows cross-compilation requires Windows system libraries"
    @echo "ERROR: Local Windows cross-compilation is not supported without Windows SDK"
    @echo ""
    @echo "Recommended alternatives:"
    @echo "  1. Use GitHub Actions for Windows builds (has Windows SDK)"
    @echo "  2. Use Docker with windows-cross image"
    @echo "  3. Build natively on Windows with 'cargo build --release'"
    @echo ""
    @echo "If you really want to try local cross-compilation:"
    @echo "  nix develop .#cross"
    @echo "  cargo build --target x86_64-pc-windows-gnu"
    @false

# Build for macOS (Intel)
build-mac:
    cargo build --release --target x86_64-apple-darwin
    mkdir -p dist/mac
    cp target/x86_64-apple-darwin/release/skit dist/mac/skit

# Build for macOS (Apple Silicon)
build-mac-arm64:
    cargo build --release --target aarch64-apple-darwin
    mkdir -p dist/mac-arm64
    cp target/aarch64-apple-darwin/release/skit dist/mac-arm64/skit

# Install required Rust targets for cross-compilation
install-targets:
    rustup target add x86_64-pc-windows-gnu
    rustup target add x86_64-apple-darwin
    rustup target add aarch64-apple-darwin
    rustup target add aarch64-unknown-linux-gnu

# Build native Linux (guaranteed to work on self-hosted Linux)
release-linux-native: build-linux
    @echo "Creating native Linux release..."
    cd dist/linux && tar -czf ../skit-linux-x86_64.tar.gz skit
    @echo "Generating checksums..."
    cd dist && sha256sum skit-linux-x86_64.tar.gz > checksums-linux.txt
    @echo "Native Linux release ready in dist/"

# Try cross-compilation (may require additional setup)
release-cross: install-targets
    @echo "⚠️  Cross-compilation may require additional linkers/toolchains"
    @echo "Attempting cross-compilation builds..."
    -just build-linux-arm64
    -just build-mac  
    -just build-mac-arm64
    @echo "Creating archives for successful builds..."
    @mkdir -p dist
    @if [ -f target/aarch64-unknown-linux-gnu/release/skit ]; then \
        mkdir -p dist/linux-arm64 && cp target/aarch64-unknown-linux-gnu/release/skit dist/linux-arm64/; \
        cd dist/linux-arm64 && tar -czf ../skit-linux-arm64.tar.gz skit; \
    fi
    @if [ -f target/x86_64-apple-darwin/release/skit ]; then \
        mkdir -p dist/mac && cp target/x86_64-apple-darwin/release/skit dist/mac/; \
        cd dist/mac && tar -czf ../skit-mac-x86_64.tar.gz skit; \
    fi
    @if [ -f target/aarch64-apple-darwin/release/skit ]; then \
        mkdir -p dist/mac-arm64 && cp target/aarch64-apple-darwin/release/skit dist/mac-arm64/; \
        cd dist/mac-arm64 && tar -czf ../skit-mac-arm64.tar.gz skit; \
    fi
    @if ls dist/*.tar.gz >/dev/null 2>&1; then \
        cd dist && sha256sum *.tar.gz > checksums-cross.txt; \
        echo "Cross-compilation artifacts ready in dist/"; \
    else \
        echo "❌ No cross-compilation succeeded. This is normal and expected."; \
    fi

# Reliable self-hosted CI pipeline (quality checks + native Linux)
ci-self-hosted: check-all test release-linux-native
    @echo "✅ Self-hosted CI pipeline completed successfully!"
    @echo "📦 Native Linux build ready for release"
    @echo "💡 Run 'just release-cross' separately if cross-compilation is set up"
    @echo "⏭️  Windows and other platforms should use GitHub Actions"

# Experimental: Try everything
ci-self-hosted-full: check-all test release-linux-native release-cross
    @echo "✅ Full self-hosted pipeline attempted!"
    @echo "📦 Check dist/ directory for available builds"


# Clean build artifacts
clean:
    cargo clean
    rm -rf dist/

# Run tests
test:
    cargo test

# Format code
fmt:
    cargo fmt

# Run clippy
clippy:
    cargo clippy

# Check code
check:
    cargo check

# Run all code quality checks (clippy, check, fmt)
check-all:
    @echo "Running cargo clippy..."
    cargo clippy
    @echo "Running cargo check..."
    cargo check
    @echo "Running cargo fmt --check..."
    cargo fmt --check
    @echo "✅ All checks passed!"

# Install skit to /usr/local/bin (requires sudo)
install:
    cargo build --release
    sudo cp target/release/skit /usr/local/bin/skit
    sudo chmod +x /usr/local/bin/skit
    @echo "skit installed to /usr/local/bin/skit"

# Prepare and publish to crates.io
publish-crate: check-all test
    @echo "🦀 Preparing to publish to crates.io..."
    @echo "📝 Make sure you've updated:"
    @echo "   - Version in Cargo.toml"
    @echo "   - CHANGELOG.md"
    @echo "   - README.md"
    @echo ""
    cargo publish --dry-run
    @echo ""
    @echo "✅ Dry run successful! Run 'cargo publish' when ready."
    @echo "📦 Users can then install with: cargo install skit"