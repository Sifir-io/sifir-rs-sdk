#!/bin/bash
# Exit on error
set -e

# Unset any iOS/macOS specific variables that might interfere
unset SDKROOT
unset PLATFORM_NAME
unset IPHONEOS_DEPLOYMENT_TARGET
unset TVOS_DEPLOYMENT_TARGET
unset XROS_DEPLOYMENT_TARGET

# First, make sure we have the targets
rustup target add \
    aarch64-linux-android \
    x86_64-linux-android \
    i686-linux-android

# Define variables
MANIFEST_PATH="./tor/Cargo.toml"
OUTPUT_DIR="target/jniLibs"

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Then, build the library
echo "Building for Android..."
cargo ndk \
    --manifest-path="$MANIFEST_PATH" \
    --platform 30 \
    -t arm64-v8a \
    -t x86 \
    -t x86_64 \
    --output-dir "$OUTPUT_DIR" \
    build --release

echo "Android build complete!"
