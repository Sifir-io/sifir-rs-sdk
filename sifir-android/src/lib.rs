#![allow(non_snake_case)]
pub mod flapigen;
pub use flapigen::java_glue::*;
pub mod logger;
use jni::objects::JClass;
use jni::JNIEnv;
use logger::logger::initLogger;

#[no_mangle]
pub unsafe extern "C" fn Java_com_sifir_sdk_Logger_initLogger(_env: JNIEnv, _: JClass) {
    initLogger();
}
