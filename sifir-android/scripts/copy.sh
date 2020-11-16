## Copy built android $targets for a $libfile to an Android project located in $DIR

# FIXME change this to input or env ? 
BASE="../app/tor"
DIR="$BASE/src/main";

# tuple (rust target,android target)
targets=("i686-linux-android" "x86"  "armv7-linux-androideabi" "armeabi" "aarch64-linux-android" "arm64" "x86_64-linux-android" "x86_64");
test_targets=("x86_64-unknown-linux-gnu" "x86_64");

libfile="libsifir_android.so";
# FIXME link this with path in build.rs
#java_gen_path="../app/src/main/java/com/sifirsdk"

# Check and crate directories in Android project
#[ ! -d "$DIR" ] && echo "Directory $DIR doesnt' exists exiting!" && exit -1;
#echo "Creating jniLibs directories in $DIR";
#mkdir -p "$DIR/jniLibs";
#retVal=$?;
#[ $retVal -ne 0 ] && echo "[ERROR] Error creating $DIR/jniLibs bugging out ..." && exit -1;

## Copy java files
#[ ! -d "$java_gen_path" ] && echo "[ERROR] Java genrated files not found in $java_gen_path, bugging out " && exit -1;
#cp -r "$java_gen_path/" "$DIR/";
#retVal=$?;
#[ $retVal -ne 0 ] && echo "[ERROR] copying files Java files from $java_gen_path to $DIR" && exit -1;

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

# Copy test targets
for ((i=0; i<${#test_targets[@]}; i+=2)); do
    libpath="../../target/${test_targets[i]}/release/$libfile";
    if [ ! -f "$libpath" ]; then
    	echo "[ERROR] $libpath couln't be found in test_targets directory skipping!";
	exit -1;
    else
	target_dir="$BASE/src/test/jniLibs/${test_targets[i+1]}";
	mkdir -p "$target_dir";
	retVal=$?;
	[ $retVal -ne 0 ] && echo "[ERROR] Error creating directories $target_dir" && exit -1;
    	cp "$libpath" "$target_dir/$libfile";
    fi
done;

echo "DONE!";
