#!/bin/bash
cargo build --target x86_64-apple-ios
cargo build --target aarch64-apple-ios
# create universal lb
lipo -create ../../target/aarch64-apple-ios/debug/libsifir_ios.a ../../target/x86_64-apple-ios/debug/libsifir_ios.a -output ../output/libsifir_ios.a
# Confirm build
lipo -info ../output/libsifir_ios.a
