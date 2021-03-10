#!/bin/bash
features=$1
under_scored_features=$(echo "$features" | tr , _)
framework_name="Libsifir_${under_scored_features}";
bundle_id="${under_scored_features}"
bundle_version="0.1.1"

export IPHONEOS_DEPLOYMENT_TARGET="11.0"

echo "---Sifir.io----";
echo "---------------";
echo "|              |";
echo "|      0       |";
echo "|              |";
echo "---------------";
echo "Will build a universal IOS static and framework:";
echo "Features: $features"
echo "Framework Name: $framework_name"
echo "---------------";
echo "---------------";

# Build local (+ FFI)
SIFIR_CBINDGEN_OUTPUT_FILENAME="$framework_name.h" cargo  build -p sifir-ios --release --features "$features";
retVal=$?
[ ! $retVal -eq 0 ] && exit 1;
SIFIR_CBINDGEN_OUTPUT_FILENAME="$framework_name.h" cargo lipo -p sifir-ios --release --features "$features"
retVal=$?
[ ! $retVal -eq 0 ] && exit 1;

# copy indiviual arch libs  for testing
mkdir -p ../output/release/universal/"$under_scored_features"
retVal=$?
[ ! $retVal -eq 0 ] && exit 1;

lib_output_target_dir="../output/release/universal/${under_scored_features}/libsifir_ios.a"

\cp -f ../../target/universal/release/libsifir_ios.a "$lib_output_target_dir"
retVal=$?
[ ! $retVal -eq 0 ] && exit 1;

echo "-------- Building Done: ${lib_output_target_dir} ------- "

# TODO add extra typedefs here via Sed?
working_dir="../output/${framework_name}/target/framework/$framework_name.framework";

mkdir -p "$working_dir/Headers";
mkdir -p "$working_dir/Modules";
\cp -f "../output/${framework_name}.h" "$working_dir/Headers/$framework_name.h"
retVal=$?
[ ! $retVal -eq 0 ] && exit 1;

lipo -create "$lib_output_target_dir" -output "$working_dir/$framework_name"
retVal=$?
[ ! $retVal -eq 0 ] && exit 1;

cat <<HERE > "$working_dir/Info.plist"
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleDevelopmentRegion</key>
	<string>en</string>
	<key>CFBundleExecutable</key>
	<string>$framework_name</string>
	<key>CFBundleIdentifier</key>
	<string>org.sifir.sifirsdk.$bundle_id</string>
	<key>CFBundleInfoDictionaryVersion</key>
	<string>6.0</string>
	<key>CFBundleName</key>
	<string>$framework_name</string>
	<key>CFBundlePackageType</key>
	<string>FMWK</string>
	<key>CFBundleShortVersionString</key>
	<string>1.0</string>
	<key>CFBundleVersion</key>
	<string>$bundle_version</string>
	<key>MinimumOSVersion</key>
	<string>11.0</string>
</dict>
</plist>
HERE

cat <<HERE > "$working_dir/Modules/module.modulemap"
framework module "$framework_name" {
    header "$framework_name.h"
    export *
}
HERE
echo "Framework building done: ${working_dir}"
echo "DONE ALL TASKS"

