#! /bin/bash
OS=$(uname)
features=$1
if [ ! "$features" ]; then
  echo "Missing features parameter"
  exit 1
fi

cd ..
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

target_dir="./outputs/cpp/$aar_dir"
export CPP_FFI_OUTPUT_DIR=$target_dir

read -p "Will build features $features to CPP target dir $target_dir, \r\n Note: This will delete everything in the target_dir parent ! \r\n Press Ctrl-C to abort or any other key to continue...";

rm -rf "$target_dir"
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


echo "--- Compiling Done! --- \r\n"

# Copy binaries to output dir
targets=("i686-linux-android" "x86"  "armv7-linux-androideabi" "armeabi-v7a" "aarch64-linux-android" "arm64"  "aarch64-linux-android" "arm64-v8a" "x86_64-linux-android" "x86_64");
test_targets=("x86_64-unknown-linux-gnu" "x86_64");
#libfile="libsifir_android.so";
libfile="libsifir_android";

# Copy lib targets to respective android project directories
for ((i=0; i<${#targets[@]}; i+=2)); do
    libpath="../target/${targets[i]}/release/$libfile";
    if [ ! -f "$libpath.so" ]; then
    	echo "[ERROR] $libpath could not be found in targets directory skipping!";
	exit 1;
    else
	libdir="$target_dir/bin/${targets[i+1]}";
	mkdir -p "$libdir";
	retVal=$?;
	[ $retVal -ne 0 ] && echo "[ERROR] Error creating directories $target_dir" && exit 1;
    	\cp -f "$libpath.d" "$libpath.so" "$libpath.a" "$libdir/";
    fi
done;
