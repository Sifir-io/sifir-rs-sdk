# Helper script to:
# 1. Copy targets to JNI libs and Copy java files to Java
# 2. Build AAR

BASE="../app/tor"
DIR="$BASE/src/main";

# tuple (rust target,android target)

targets=("i686-linux-android" "x86"  "armv7-linux-androideabi" "armeabi-v7a" "aarch64-linux-android" "arm64"  "aarch64-linux-android" "arm64-v8a" "x86_64-linux-android" "x86_64");
test_targets=("x86_64-unknown-linux-gnu" "x86_64");
libfile="libsifir_android.so";

# Copy lib targets to respevtive android project directories 
for ((i=0; i<${#targets[@]}; i+=2)); do
    libpath="../../target/${targets[i]}/release/$libfile";
    if [ ! -f "$libpath" ]; then
    	echo "[ERROR] $libpath could not be found in targets directory skipping!";
	exit 1;
    else
	target_dir="$DIR/jniLibs/${targets[i+1]}";
	mkdir -p "$target_dir";
	retVal=$?;
	[ $retVal -ne 0 ] && echo "[ERROR] Error creating directories $target_dir" && exit 1;
    	\cp -f "$libpath" "$target_dir/$libfile";
    fi
done;

# Copy test targets
for ((i=0; i<${#test_targets[@]}; i+=2)); do
    libpath="../../target/${test_targets[i]}/release/$libfile";
    if [ ! -f "$libpath" ]; then
    	echo "[ERROR] $libpath could not be found in test_targets directory skipping!";
	exit 1;
    else
	target_dir="$BASE/src/test/jniLibs/${test_targets[i+1]}";
	mkdir -p "$target_dir";
	retVal=$?;
	[ $retVal -ne 0 ] && echo "[ERROR] Error creating directories $target_dir" && exit 1;
	\cp -f "$libpath" "$target_dir/$libfile";
    fi
done;
echo "Copied all binaries...";

# Build AAR
cd ../app && ./gradlew assembleRelease
[ $retVal -ne 0 ] && echo "[ERROR] Building AAR" && exit 1;
cd ../scripts
\cp -rf ../app/tor/build/outputs/aar ../outputs/
echo "AAR built!";
du -d 1 -h  ../outputs/aar/
