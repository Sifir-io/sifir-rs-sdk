#! /bin/bash
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

export SIFIR_ANDROID_JAVA_DIR=$aar_dir
target_dir="./app/tor/src/main/java/com/sifir/$SIFIR_ANDROID_JAVA_DIR"

read -p "Will build features $features to aar target dir $target_dir, press Ctrl-C to abort or any other key to continue...";


mkdir -p "$target_dir"
retVal=$?
[ ! $retVal -eq 0 ] && exit 1

if [ "$OS" = "Darwin" ]; then
  echo "building apple darwin x86_64 lib"
  cargo build --target x86_64-apple-darwin -p sifir-android --release --features "$features"
  retVal=$?

  [ ! $retVal -eq 0 ] && exit 1
elif [ "$OS" = "Linux" ]; then
  echo "building linux x86_64 lib"
  cargo build --target x86_64-unknown-linux-gnu -p sifir-android --release --features "$features"
  retVal=$?
  [ ! $retVal -eq 0 ] && exit 1
fi

cargo ndk --platform 30 --target x86_64-linux-android build --release --features "$features"
retVal=$?
[ ! $retVal -eq 0 ] && exit 1
cargo ndk --platform 30 --target aarch64-linux-android build -p sifir-android --release --features "$features"
retVal=$?
[ ! $retVal -eq 0 ] && exit 1
cargo ndk --platform 30 --target armv7-linux-androideabi build -p sifir-android --release --features "$features"
retVal=$?
[ ! $retVal -eq 0 ] && exit 1
cargo ndk --platform 30 --target i686-linux-android build -p sifir-android --release --features "$features"
retVal=$?
[ ! $retVal -eq 0 ] && exit 1

echo "Done!"
