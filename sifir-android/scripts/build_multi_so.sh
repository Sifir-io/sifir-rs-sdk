#! /bin/bash
#CXX_x86_64_linux_android=$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/x86_64-linux-android29-clang++
#CXX_aarch64_linux_android=$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android29-clang++
#CXX_armv7_linux_androideabi=$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/armv7a-linux-androideabi29-clang++
#CXX_i686_linux_android=$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/i686-linux-android29-clang++
#cd ..

OS=`uname`

if [ "$OS" = "Darwin" ]
then
  echo "building apple darwin x86_64 lib"
  cargo build --target x86_64-apple-darwin -p sifir-android --release #--features "java"
retVal=$?
[ ! $retVal -eq 0 ] && exit 1;
elif [ "$OS" = "Linux" ]
then
  echo "building linux x86_64 lib"
  cargo build --target x86_64-unknown-linux-gnu -p sifir-android --release #--features "java"
retVal=$?
[ ! $retVal -eq 0 ] && exit 1;
fi

cargo ndk --platform 30 --target x86_64-linux-android build --release
retVal=$?
[ ! $retVal -eq 0 ] && exit 1;
cargo ndk --platform 30 --target aarch64-linux-android build -p sifir-android --release
retVal=$?
[ ! $retVal -eq 0 ] && exit 1;
cargo ndk --platform 30 --target armv7-linux-androideabi build -p sifir-android --release
retVal=$?
[ ! $retVal -eq 0 ] && exit 1;
cargo ndk --platform 30 --target i686-linux-android build -p sifir-android --release
retVal=$?
[ ! $retVal -eq 0 ] && exit 1;

echo "Done!"
