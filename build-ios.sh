#!/bin/bash

# Exit on error
set -e

# Set up iOS-specific environment
unset PLATFORM_NAME
unset DEVELOPER_DIR
unset SDKROOT
unset IPHONEOS_DEPLOYMENT_TARGET
unset TVOS_DEPLOYMENT_TARGET
unset XROS_DEPLOYMENT_TARGET
unset MACOSX_DEPLOYMENT_TARGET
export IPHONEOS_DEPLOYMENT_TARGET="16.0"
export PLATFORM_NAME=iphoneos
export DEVELOPER_DIR="$(xcode-select -p)"
export SDKROOT="$DEVELOPER_DIR/Platforms/iPhoneSimulator.platform/Developer/SDKs/iPhoneSimulator.sdk"

# First, make sure we have the targets
rustup target add \
    x86_64-apple-ios \
    aarch64-apple-ios \
    aarch64-apple-ios-sim

# Then, build the library
TARGET_DIR="target/ios"
MANIFEST_PATH="./tor/Cargo.toml"
BINARY_NAME="libtor.a"
CXX_BINARY_PATH_IOS="target/ios/aarch64-apple-ios/release/build/cxx-d1bd1d29a6dcc11a/out/libcxxbridge1.a"
CXX_BINARY_PATH_IOS_SIM="target/ios/aarch64-apple-ios-sim/release/build/cxx-f28d8b7ce9bb5b34/out/libcxxbridge1.a"

mkdir -p $TARGET_DIR

echo "Building for iOS (arm64)..."
cargo build --release \
    --manifest-path="$MANIFEST_PATH" \
    --target aarch64-apple-ios \
    --target-dir $TARGET_DIR

# echo "Building for iOS (x86_64)..."
# cargo build --release \
#     --manifest-path="$MANIFEST_PATH" \
#     --target x86_64-apple-ios \
#     --target-dir "$TARGET_DIR"

echo "Building for iOS (aarch64-sim)..."
cargo build --release \
    --manifest-path="$MANIFEST_PATH" \
    --target aarch64-apple-ios-sim \
    --target-dir $TARGET_DIR

# Create universal binary (optional)
# echo "Creating universal binary..."
# if [ -f "target/ios/aarch64-apple-ios/release/libsifir_ios.a" ] && \
#    [ -f "target/ios/aarch64-apple-ios-sim/release/libsifir_ios.a" ]; then
#     mkdir -p target/ios/universal
#     lipo -create \
#         target/ios/aarch64-apple-ios/release/libsifir_ios.a \
#         target/ios/aarch64-apple-ios-sim/release/libsifir_ios.a \
#         -output target/ios/universal/libsifir_ios.a
#     echo "Universal binary created at target/ios/universal/libsifir_ios.a"
# else
#     echo "Warning: Could not create universal binary. One or both architecture builds are missing."
# fi

# echo "iOS build complete!"


# Create temporary directories for the frameworks
mkdir -p target/ios/ios-device/Headers target/ios/ios-simulator/Headers

rm -rf target/ios/Tor.xcframework target/ios/TorCxxBridge.xcframework
HEADERS_DIR_IOS="target/ios/ios-device/Headers"
HEADERS_DIR_IOS_SIM="target/ios/ios-simulator/Headers"

# Create the framework structures
# Create XCFramework for the main library
xcodebuild -create-xcframework \
  -library target/ios/aarch64-apple-ios/release/$BINARY_NAME \
  -headers $HEADERS_DIR_IOS \
  -library target/ios/aarch64-apple-ios-sim/release/$BINARY_NAME \
  -headers $HEADERS_DIR_IOS_SIM \
  -output target/ios/Tor.xcframework

# Create XCFramework for the cxxbridge library
xcodebuild -create-xcframework \
  -library $CXX_BINARY_PATH_IOS \
  -headers $HEADERS_DIR_IOS \
  -library $CXX_BINARY_PATH_IOS_SIM \
  -headers $HEADERS_DIR_IOS_SIM \
  -output target/ios/TorCxxBridge.xcframework
