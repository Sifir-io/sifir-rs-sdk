#[macro_use]
extern crate lazy_static;
use std::panic::catch_unwind;
mod tor;

// use std::ffi::{size_t};
// use std::os::raw::c_char;

#[repr(C)]
pub struct RustByteSlice {
    pub bytes: *const u8,
    pub len: usize,
}

#[no_mangle]
pub extern "C" fn send_request() -> RustByteSlice {
    match catch_unwind(|| {
        let s = "I am from RUST!";
        let mut tor = tor::TorService::new();
        let _ = tor.start_service();
        //    let client = tor.get_client().unwrap();
        //    let resp = client
        //        .get("http://keybase5wmilwokqirssclfnsqrjdsi7jdir5wy7y7iu3tanwmtp6oid.onion")
        //        .send()
        //        .await.unwrap();
        //client.ge
        s
    }) {
        Ok(s) => RustByteSlice {
            bytes: s.as_ptr(),
            len: s.len() as usize,
        },
        Err(_) => {
            let err = "%%ERRROR%%";
            RustByteSlice {
                bytes: err.as_ptr(),
                len: err.len() as usize,
            }
        }
    }
}
