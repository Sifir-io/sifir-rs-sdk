#! /bin/bash
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

cargo ndk --android-platform 30 --target x86_64-linux-android -- build -p sifir-android --release
retVal=$?
[ ! $retVal -eq 0 ] && exit 1;
cargo ndk --android-platform 30 --target aarch64-linux-android -- build -p sifir-android --release
retVal=$?
[ ! $retVal -eq 0 ] && exit 1;
cargo ndk --android-platform 30 --target armv7-linux-androideabi -- build -p sifir-android --release
retVal=$?
[ ! $retVal -eq 0 ] && exit 1;
cargo ndk --android-platform 30 --target i686-linux-android -- build -p sifir-android --release
retVal=$?
[ ! $retVal -eq 0 ] && exit 1;

echo "Done!"
