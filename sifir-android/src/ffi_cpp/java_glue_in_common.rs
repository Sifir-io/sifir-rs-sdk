use jni_sys::*;
use logger::{log, Logger};
use log::*;

use serde_json::json;

foreign_class!(class Logger {
    self_type Logger;
    private constructor = empty;
    fn Logger::new()->Logger;
});
