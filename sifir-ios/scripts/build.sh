#!/bin/bash
if [ ! -z $1 ]; then
	target=$1;
fi

cargo build -p sifir-ios --target x86_64-apple-ios --"$target";
cargo build -p sifir-ios --target aarch64-apple-ios --"$target";

[ -z $target ] && target="debug";

mkdir -p ../output/"$target"/{universal,aarch64-apple-ios,x86_64-apple-ios};

# copy indiviual libraries here (TODO we can remove this once we get universal binary under control)
cp ../../target/aarch64-apple-ios/"$target"/libsifir_ios.a ../output/"$target"/aarch64-apple-ios/libsifir_ios.a
cp ../../target/x86_64-apple-ios/"$target"/libsifir_ios.a ../output/"$target"/x86_64-apple-ios/libsifir_ios.a

# create universal lb
lipo -create ../../target/aarch64-apple-ios/"$target"/libsifir_ios.a ../../target/x86_64-apple-ios/"$target"/libsifir_ios.a -output ../output/"$target"/universal/libsifir_ios.a

# Confirm universal binary fat
lipo -info ../output/"$target"/universal/libsifir_ios.a
