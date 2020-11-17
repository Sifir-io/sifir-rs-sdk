use cbindgen::Language::C;
use std::env;
fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_language(C)
        .generate()
        .expect("Unable to generate bindings!!")
        .write_to_file("./output/sifir-tor.h");
}
