# sifir-rs-sdk

Builds a universal dyanamic library for iOS and a static library for Android.


- All of the code is in the `tor` directory.
- The other directories for `sifir-android` and `sifir-ios` and no longer used.
- Check the build script `build-android.sh` for building for android.
- We build both `cdylib` and `staticlib`, but we only copy `.a` files to the `jniLibs` directory.
- If you want the `.so` files, just go the inidividual target directories.

### How this works:

- The raw `tor_api.h` file is built using a library called [libtor-sys](https://github.com/niteshbalusu11/libtor-sys). This is super low level.
- Then there is a little wrapper library called [libtor](https://github.com/niteshbalusu11/libtor) that exposes a nicer Rust API to start a tor instance.
- The `tor` crate in this repo takes the `libtor` library and then adds more functionality as well as cxx ffi bindings.
- All of these are forks of the original projects with some dependency bumps so that they can actually work in 2025.

```
# Build Android
./build-android.sh

# Build iOS
./build-ios.sh
```

## Supported platforms

* Android through the NDK (API level 30+)
* MacOS
* iOS

### Build using nix on a Mac
- Install [nix](https://determinate.systems/nix-installer/)
- Install [direnv](https://direnv.net/)
- Run `direnv allow` to allow direnv to load the nix environment
- If you want to install xcode and xcode command line tools, simply run `setup-ios-env`.

```
# Build Android
build-android

# Build iOS
build-ios

# Build MacOS
build-macos

# Build all
build-all
```
