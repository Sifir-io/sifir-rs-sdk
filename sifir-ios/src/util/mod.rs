use libc::c_char;
use logger;

#[no_mangle]
/// Starts env logger
pub extern "C" fn start_logger() {
    logger::Logger::new();
}

#[repr(C)]
pub enum ResultMessage {
    Success,
    Error(*mut c_char),
}
#[repr(C)]
pub struct BoxedResult<T> {
    pub result: Option<Box<T>>,
    pub message: ResultMessage,
}

#[no_mangle]
///# Safety
/// Destroy a cstr
pub unsafe extern "C" fn destroy_cstr(c_str: *mut c_char) {
    assert!(!c_str.is_null());
    let _ = Box::from_raw(c_str);
}
