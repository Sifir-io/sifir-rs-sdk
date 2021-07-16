use flapigen::*;
use std::env;
use std::path::Path;

fn main() {
    // TODO this comes from env because script creates directory
    // Move it all here so we can create and manage all features in one place
    let outputname =
        env::var("SIFIR_ANDROID_JAVA_DIR").expect("Missing SIFIR_ANDROID_JAVA_DIR env for target");
    setup_java(outputname);
}
fn setup_java(target: String) {
    println!("Generate bindings...");
    let mut in_src_vec = vec![Path::new("src/ffi/").join("java_glue_in_common.rs")];

    #[cfg(feature = "tor_daemon")]
    in_src_vec.push(Path::new("src/ffi/").join("java_glue_in_tor.rs"));

    #[cfg(feature = "btc_wallet")]
    in_src_vec.push(Path::new("src/ffi/").join("java_glue_in_btc.rs"));

    let out_src = Path::new("src/ffi/").join("java_glue.rs");
    //ANCHOR: config
    let swig_gen = flapigen::Generator::new(LanguageConfig::JavaConfig(
        JavaConfig::new(
            Path::new("app")
                .join("tor")
                .join("src")
                .join("main")
                .join("java")
                .join("com")
                .join("sifir")
                .join(target.clone()),
            format!("com.sifir.{}", target),
        ), // .use_null_annotation_from_package("android.support.annotation".into()),
    ))
    .rustfmt_bindings(true);
    //ANCHOR_END: config
    swig_gen.expand_many("android bindings", &in_src_vec, &out_src);
}
