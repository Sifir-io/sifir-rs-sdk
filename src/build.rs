use std::env;
extern crate cbindgen;
extern crate lipo;

use cbindgen::Language::C;

fn main() {
        setup_cbindgen();
}

fn setup_cbindgen() {
        let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
            cbindgen::Builder::new() .with_crate(crate_dir) .with_language(C) .with_include("swapi_core.h") .generate() .expect("Unable to generate bindings") .write_to_file("src/ffi/swapi_native.h"); }
