use libc::{c_char, c_void};
use std::ffi::{CStr, CString};
use std::panic::catch_unwind;

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
