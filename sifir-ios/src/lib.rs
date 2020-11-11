//#[macro_use]
//extern crate lazy_static;
//use std::panic::catch_unwind;
//// mod tor;
use crate::tor::{TorService,TorServiceParam};
//
//// use std::ffi::{size_t};
//// use std::os::raw::c_char;
//// let rt = tokio::runtime::Builder::new();
//use std::ffi::CString;
//use std::os::raw::{c_char, c_void};
//use std::ops::Deref;
//use super::callback::{Callback};
//
//
///// METHOD 1
///// Using Pointers to send the actual client back to Swift
///// and have it do whatever it need to do there, we just destory etc.
//
////Create client
//#[no_mangle]
//pub extern "C" fn create_tor_client() -> *mut TorService{
//    Box::into_raw(Box::new(TorService::new()))
//}
//
////Release memory
//#[no_mangle]
//pub unsafe extern "C" fn destory_tor_client(client: *mut TorService) {
//    assert!(!client.is_null());
//    Box::from_raw(client);
//}
//
//
///// --- METHOD 2
///// Using a byteSlice to retun values
///// FIXME How do we keep the Service on ?
//#[repr(C)]
//pub struct RustByteSlice {
//    pub bytes: *const u8,
//    pub len: usize,
//}
//#[no_mangle]
//pub extern "C" fn start_service() -> RustByteSlice {
//    match catch_unwind(|| {
//        let s = "I am from RUST!";
//        s
//    }) {
//        Ok(s) => RustByteSlice {
//            bytes: s.as_ptr(),
//            len: s.len() as usize,
//        },
//        Err(_) => {
//            let err = "%%ERRROR%%";
//            RustByteSlice {
//                bytes: err.as_ptr(),
//                len: err.len() as usize,
//            }
//        }
//    }
//}
//
