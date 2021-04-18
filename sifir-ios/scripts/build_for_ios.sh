#!/bin/bash
echo "---Sifir.io----";
echo "---------------";
echo "|              |";
echo "|      0       |";
echo "|              |";
echo "---------------";
echo "Will build a universal IOS dylib !!";
echo "---------------";
echo "---------------";

# Build local (+ FFI)
cargo  build -p sifir-ios --release;
retVal=$?
[ ! $retVal -eq 0 ] && exit 1;

export IPHONEOS_DEPLOYMENT_TARGET="11.0"

cargo lipo -p sifir-ios --release
retVal=$?
[ ! $retVal -eq 0 ] && exit 1;

# copy indiviual arch libs  for testing
\cp -f ../../target/universal/release/libsifir_ios.a ../output/release/universal/libsifir_ios.a

# Update dylib rpath
# install_name_tool -id "@rpath/libsifir_ios.dylib" ../output/"$target"/universal/libsifir_ios.dylib

echo "Done!":

