## Copy built android $targets for a $libfile to an Android project located in $DIR

# FIXME change this to input or env ? 
DIR="/home/gus/Projects/sifir-io-public/rn-tor/android/src/main/java/com/reactnativerntor";

# tuple (rust target,android target)
targets=("i686-linux-android" "x86"  "armv7-linux-androideabi" "armeabi" "aarch64-linux-android" "arm64");
libfile="libsifir_android.so";
# FIXME link this with path in build.rs
java_gen_path="../app/src/main/java/com/sifir/sdk"

# Check and crate directories in Android project
[ ! -d "$DIR" ] && echo "Directory $DIR doesnt' exists exiting!" && exit -1;
echo "Creating jniLibs directories in $DIR";
mkdir -p "$DIR/jniLibs";
retVal=$?;
[ $retVal -ne 0 ] && echo "[ERROR] Error creating $DIR/jniLibs bugging out ..." && exit -1;

# Copy java files
[ ! -d "$java_gen_path" ] && echo "[ERROR] Java genrated files not found in $java_gen_path, bugging out " && exit -1;
mkdir -p "$DIR/sifir_sdk";
retVal=$?;
[ $retVal -ne 0 ] && echo "[ERROR] Error $DIR/sifir_sdk bugging out..." && exit -1;
cp -r "$java_gen_path/" "$DIR/";

# Copy lib targets to respevtive android project directories 
for ((i=0; i<${#targets[@]}; i+=2)); do
    libpath="../../target/${targets[i]}/release/$libfile";
    if [ ! -f "$libpath" ]; then
    	echo "[ERROR] $libpath couln't be found in targets directory skipping!";
	exit -1;
    else
	target_dir="$DIR/jniLibs/${targets[i+1]}";
	mkdir -p "$target_dir";
	retVal=$?;
	[ $retVal -ne 0 ] && echo "[ERROR] Error creating directories $target_dir" && exit -1;
    	cp "$libpath" "$target_dir/$libfile";
    fi
done;

echo "DONE!";
