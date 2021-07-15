use flapigen::*;
use std::env;
use std::path::Path;

fn main() {
    let outputname = env::var("SIFIR_ANDROID_JAVA_DIR").unwrap_or(String::from("tor"));
    setup_java(outputname);
}
fn setup_java(target: String) {
    println!("Generate tor daemon bindings...");
    let in_src = Path::new("src/ffi/").join("java_glue_in.rs");
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
            format!("com.sifir.{}", target).into(),
        ), // .use_null_annotation_from_package("android.support.annotation".into()),
    ))
    .rustfmt_bindings(true);
    //ANCHOR_END: config
    swig_gen.expand("android bindings", &in_src, &out_src);
    println!("cargo:rerun-if-changed={}", in_src.display());
}
