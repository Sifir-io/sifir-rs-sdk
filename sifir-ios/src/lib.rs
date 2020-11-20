use libc::{c_char, strlen};
use std::panic::catch_unwind;
use std::{slice, str};
use tor::{OwnedTorService, TorServiceParam};
use std::ffi::{CStr,CString};

#[repr(C)]
pub struct RustByteSlice {
    pub bytes: *const u8,
    pub len: usize,
}

impl From<String> for RustByteSlice {
    fn from(s: String) -> RustByteSlice {
        RustByteSlice {
            bytes: s.as_ptr(),
            len: s.len() as usize,
        }
    }
}


#[repr(C)]
enum ResultMessage {
    Success,
    Error(RustByteSlice),
}
#[repr(C)]
/// Since the FFI simply starts and shutdowns the daemon we use an
/// Opaque pointer here to pass across the FFI
pub struct BoxedResult<T> {
    result: Option<Box<T>>,
    message: ResultMessage,
}
#[no_mangle]
pub extern "C" fn get_owned_TorService(
    data_dir: *const c_char,
    socks_port: u16,
) -> *mut BoxedResult<OwnedTorService> {
    match catch_unwind(|| {
        assert!(!data_dir.is_null());
        let dir_str:String = unsafe {
            CStr::from_ptr(data_dir)
        }.to_str().expect("Could not get str from data_dir").to_owned();

        println!("Starting TorService with Datadir {} SocksPort {}",dir_str,socks_port);
        let param = TorServiceParam {
            socks_port: Some(socks_port),
            data_dir: dir_str,
        };
        OwnedTorService::new(param)
    }) {
        Ok(service) => Box::into_raw(Box::new(BoxedResult {
            result: Some(Box::new(service)),
            message: ResultMessage::Success,
        })),
        Err(e) => {
            let message:RustByteSlice = match e.downcast::<String>() {
                Ok(msg) => *msg,
                Err(_) => String::from("Unknown panic"),
            }.into();

            Box::into_raw(Box::new(BoxedResult {
                result: None,
                message: ResultMessage::Error(message),
            }))
        }
    }
}
//
#[no_mangle]
///# Safety
/// Destroy and release ownedTorBox which will shut down owned connection and shutdown daemon
pub unsafe extern "C" fn shutdown_owned_TorService(
    owned_client: *mut OwnedTorService,
) {
    assert!(!owned_client.is_null());
    let mut owned: Box<OwnedTorService> = Box::from_raw(owned_client);
    owned.shutdown();
}
