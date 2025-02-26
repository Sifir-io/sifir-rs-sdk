fn main() {
    let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();

    // Generate C/C++ headers
    cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_config(
            cbindgen::Config::from_file("cbindgen.toml").expect("Failed to load cbindgen.toml"),
        )
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file("tor_ffi.h");

    println!("cargo:rerun-if-changed=src/ffi.rs");
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=cbindgen.toml");
    println!("cargo:rustc-link-lib=tor");
}
