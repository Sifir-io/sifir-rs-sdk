#![allow(non_snake_case)]
pub mod flapigen;
pub use flapigen::java_glue::*;
pub mod logger;
use logger::logger::initLogger;
use jni::JNIEnv;
use jni::objects::JClass;

#[no_mangle]
pub unsafe extern fn Java_com_rust_app_swapiclient_swapi_Logger_initLogger(_env: JNIEnv, _: JClass) {
    initLogger();
}
