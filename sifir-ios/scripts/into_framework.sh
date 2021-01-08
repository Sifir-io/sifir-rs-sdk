#!/bin/bash
echo "---Sifir.io----";
echo "---------------";
echo "|              |";
echo "|      0       |";
echo "|              |";
echo "---------------";
echo "Will build framework from dylib output";
echo "---------------";
echo "---------------";

target="release";
framework_name="Libsifir_ios";
bundle_id="tor"
# TODO get this from cargo
bundle_version="0.1.1"
# TODO add extra typedefs here via Sed?
working_dir="../output/$target/framework/$framework_name.framework";

mkdir -p "$working_dir/Headers";
mkdir -p "$working_dir/Modules";
\cp -f "../output/sifir-tor.h" "$working_dir/Headers/$framework_name.h"
\cp -f "../output/sifir_typedef.h" "$working_dir/Headers/sifir_typedef.h"
retVal=$?
[ ! $retVal -eq 0 ] && exit 1;
# FIXME THIS
#sed -r -i 's/[^\w](OwnedTorService)\s+/\1_t/g' "$working_dir/Headers/$framework_name.h"
lipo -create "../output/$target/universal/libsifir_ios.dylib" -output "$working_dir/$framework_name"
retVal=$?
[ ! $retVal -eq 0 ] && exit 1;
install_name_tool -id "@rpath/$framework_name.framework/$framework_name" "$working_dir/$framework_name"
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
echo "Done!":

