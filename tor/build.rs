fn main() {
    cxx_build::bridge("src/ffi.rs")
        .flag_if_supported("-std=c++14")
        .flag_if_supported("-fexceptions") // Enable exceptions
        .flag_if_supported("-frtti") // Enable RTTI
        .flag_if_supported("-fPIC") // Position Independent Code for dynamic libs
        .shared_flag(true) // Enable shared library building
        .compile("cxxbridge1");

    println!("cargo:rerun-if-changed=src/ffi.rs");
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rustc-link-lib=tor");
}
