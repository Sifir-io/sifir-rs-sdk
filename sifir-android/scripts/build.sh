#! /bin/bash
CXX_x86_64_linux_android=$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/x86_64-linux-android29-clang++
CXX_aarch64_linux_android=$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android29-clang++
CXX_armv7_linux_androideabi=$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/armv7a-linux-androideabi29-clang++
CXX_i686_linux_android=$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/i686-linux-android29-clang++
cd ..
echo "building android x86_64 lib"
cargo ndk --platform 30 --target x86_64-linux-android build --release #--features "android"

echo "building android aarch64 lib"
cargo ndk --platform 30 --target aarch64-linux-android build --release #--features "android"

echo "building android armv7 lib"
cargo ndk --platform 30 --target armv7-linux-androideabi build --release #--features "android"

echo "building android i686 lib"
cargo ndk --platform 30 --target i686-linux-android build --release #--features "android"
echo built!
#
#CC=aarch64-linux-android21-clang cargo build --target aarch64-linux-android --release
#CC=armv7a-linux-androideabi21-clang cargo build --target armv7-linux-androideabi --release
#CC=i686-linux-android21-clang cargo build --target i686-linux-android --release
