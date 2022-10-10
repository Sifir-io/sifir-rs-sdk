// use cbindgen::Language::C;
use std::env;
fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let outputname =
        env::var("SIFIR_CBINDGEN_OUTPUT_FILENAME").unwrap_or(String::from("sifir-tor.h"));

    cbindgen::generate(crate_dir)
        .unwrap()
        .write_to_file(format!("./output/{}", outputname));
}
