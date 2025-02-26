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

```rust
use std::convert::TryInto;

use tor::{TorHiddenServiceParam, TorService, TorServiceParam};

fn main() {
    println!("---------------");
    println!("Sifir - Hidden Service and Proxy Creator !");
    println!("This will create a hidden service that forwards incoming connections to a port of your choosing");
    println!("---------------");
    let hs_port: u16 = 20011;
    let socks_port: u16 = 19054;
    let service: TorService = TorServiceParam {
        socks_port: Some(socks_port),
        data_dir: String::from("/tmp/sifir_rs_sdk/"),
        bootstrap_timeout_ms: Some(45000),
    }
    .try_into()
    .unwrap();
    println!("---------Starting Tor Daemon and Socks Port ------");
    let mut owned_node = service.into_owned_node().unwrap();
    loop {
        println!("Enter a port to foward onion:");
        let mut port = String::new();
        std::io::stdin().read_line(&mut port).unwrap();
        let to_port: u16 = port.trim().parse::<u16>().unwrap();
        let service_key = owned_node
            .create_hidden_service(TorHiddenServiceParam {
                to_port,
                hs_port,
                secret_key: None,
            })
            .unwrap();

        let mut onion_url =
            utils::reqwest::Url::parse(&format!("http://{}", service_key.onion_url)).unwrap();
        let _ = onion_url.set_port(Some(hs_port));
        println!(
        "Hidden Service Created!!\n Hidden Service Onion URL: {}\n Forwarding to Port: {}\n Socks5 Proxy: 127.0.0.1:{}\n",
        onion_url, to_port,socks_port
        );

        // TODO write keys + param to file and on open if found prompt to restore

        println!("Press \"h\" to add a new service or any other key to exit");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        match input.trim() {
            "h" => continue,
            _ => return,
        }
    }
}
```
