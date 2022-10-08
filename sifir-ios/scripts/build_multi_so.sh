#! /bin/bash
# export ANDROID_NDK_HOME=$ANDROID_HOME/ndk/23.1.7779620
cd ..


OS=$(uname)
features=$1
if [ ! "$features" ]; then
  echo "Missing features parameter"
  exit 1
fi

# TODO better way to do this
case $features in
"btc_wallet")
  aar_dir="btc"
  ;;
"tor_daemon")
  aar_dir="tor"
  ;;
"btc_wallet,tor_daemon")
  aar_dir="tor_btc"
  ;;
"tor_daemon,btc_wallet")
  aar_dir="tor_btc"
  ;;
*)
  echo "unknown target combo!"
  exit 1;
esac


if [ "$OS" = "Darwin" ]; then
  echo "building apple darwin x86_64 lib"
  cargo build --target x86_64-apple-darwin -p sifir-ios --release --features "$features"
  retVal=$?

  [ ! $retVal -eq 0 ] && exit 1
elif [ "$OS" = "Linux" ]; then
  echo "building linux x86_64 lib"
  cargo build --target x86_64-unknown-linux-gnu -p sifir-ios --release --features "$features"
  retVal=$?
  [ ! $retVal -eq 0 ] && exit 1
fi

cargo ndk --platform 30 --target x86_64-linux-android --target aarch64-linux-android --target armv7-linux-androideabi --target i686-linux-android --output-dir ./output/jniLibs build --release  --features "$features"
retVal=$?
[ ! $retVal -eq 0 ] && exit 1

echo "Done!"
