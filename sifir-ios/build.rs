use cbindgen::Language::C;
use std::env;
fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let outputname =
        env::var("SIFIR_CBINDGEN_OUTPUT_FILENAME").unwrap_or(String::from("sifir-tor.h"));

    cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_language(C)
        .with_define("feature", "tor_daemon", "TOR_DAEMON")
        .with_define("feature", "btc_wallet", "BTC_WALLET")
        .generate()
        .expect("Unable to generate bindings!!")
        .write_to_file(format!("./output/{}", outputname));
}
