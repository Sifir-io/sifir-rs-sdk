use libc::{strlen,c_char};
use std::ffi::CStr;
use tor::{OwnedTorService, TorServiceParam};
use std::{slice, str};

#[repr(C)]
/// Since the FFI simply starts and shutdowns the daemon we use an
/// Opaque pointer here to pass across the FFI
pub struct OwnedTorBoxed {
    service: Option<Box<OwnedTorService>>,
}
#[no_mangle]
pub extern "C" fn get_owned_TorService(
    data_dir: *const c_char,
    socks_port: u16,
) -> *mut OwnedTorBoxed {
    let dir_str = unsafe {
        str::from_utf8_unchecked(slice::from_raw_parts(data_dir as *const u8, strlen(data_dir)+1))
    };
    let param = TorServiceParam {
        socks_port: Some(socks_port),
        data_dir: dir_str.into()
    };
    Box::into_raw(Box::new(OwnedTorBoxed {
        service: Some(Box::new(OwnedTorService::new(param))),
    }))
}
//
#[no_mangle]
///# Safety
/// Destroy and release ownedTorBox which will shut down owned connection and shutdown daemon
pub unsafe extern "C" fn shutdown_owned_TorService(owned_client: *mut OwnedTorBoxed) {
    assert!(!owned_client.is_null());
    let mut owned: Box<OwnedTorBoxed> = Box::from_raw(owned_client);
    owned.service.unwrap().shutdown();
}
