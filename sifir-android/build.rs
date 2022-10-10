use flapigen::*;
use std::env;
use std::path::Path;

fn main() {
    // TODO this comes from env because script creates directory
    // Move it all here so we can create and manage all features in one place
    //    setup_android();
    setup_cpp();
}
fn setup_cpp() {
    println!("Generate CPP bindings...");
    let out = env::var("CPP_FFI_OUTPUT_DIR").expect("Missing CPP_FFI_OUTPUT_DIR");
    let out_src = Path::new(".").join(out.as_str());

    //println!(
    //    "{:#?}",
    //    out_src
    //        .read_dir()
    //        .unwrap()
    //        .map(|dir| format!("{:#?}", dir.unwrap()))
    //        .collect::<String>()
    //);
    let mut in_src_vec = vec![Path::new("src/ffi_cpp/").join("java_glue_in_common.rs")];

    #[cfg(feature = "tor_daemon")]
    in_src_vec.push(Path::new("src/ffi_cpp/").join("java_glue_in_tor.rs"));

    #[cfg(feature = "btc_wallet")]
    in_src_vec.push(Path::new("src/ffi_cpp/").join("java_glue_in_btc.rs"));

    //ANCHOR: config
    let swig_gen = flapigen::Generator::new(LanguageConfig::CppConfig(
        CppConfig::new(out_src.clone(), "sifir_lib".into())
            .cpp_optional(CppOptional::Boost)
            .cpp_variant(CppVariant::Boost)
            .cpp_str_view(CppStrView::Boost),
    ))
    .rustfmt_bindings(true);
    //ANCHOR_END: config
    swig_gen.expand_many(
        "c++-api-for-sifir",
        &in_src_vec,
        Path::new("src/ffi_cpp/").join("java_glue.rs"),
    );
}
fn setup_android() {
    println!("Generate Android bindings...");
    let target =
        env::var("SIFIR_ANDROID_JAVA_DIR").expect("Missing SIFIR_ANDROID_JAVA_DIR env for target");

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
