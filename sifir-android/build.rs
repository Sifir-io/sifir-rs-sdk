use flapigen::*;
use std::path::Path;

fn main() {
    setup_java();
}
fn setup_java() {
    println!("Building Java bidingins");
    let in_src = Path::new("src/flapigen/").join("java_glue_in.rs");
    let out_src = Path::new("src/flapigen/").join("java_glue.rs");
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
                // FIXME this depending on cfg
                .join("tor"),
            "com.sifir.tor".into(),
        ), // .use_null_annotation_from_package("android.support.annotation".into()),
    ))
    .rustfmt_bindings(true);
    //ANCHOR_END: config
    swig_gen.expand("android bindings", &in_src, &out_src);
    println!("cargo:rerun-if-changed={}", in_src.display());
}
