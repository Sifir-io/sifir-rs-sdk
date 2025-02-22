fn main() {
    cxx_build::bridge("src/ffi.rs") // Assumes your CXX bridge code is in lib.rs
        .flag_if_supported("-std=c++14")
        .compile("tor-bindings");

    // Let cargo know to regenerate if these files change
    println!("cargo:rerun-if-changed=src/lib.rs");

    // Add any system dependencies required by libtor
    println!("cargo:rustc-link-lib=tor");

    // Tell cargo where to find the library
    println!(
        "cargo:rustc-link-search=native={}",
        std::env::var("OUT_DIR").unwrap()
    );
}
