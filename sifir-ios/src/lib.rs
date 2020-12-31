use libc::{c_char, strlen};
use serde::{Deserialize, Serialize};
use std::ffi::{CStr, CString};
use std::panic::catch_unwind;
use std::str;
use tor::{MsgOverTcp, OwnedTorService, TorServiceParam};

#[repr(C)]
enum ResultMessage {
    Success,
    Error(*mut c_char),
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
        let dir_str: String = unsafe { CStr::from_ptr(data_dir) }
            .to_str()
            .expect("Could not get str from data_dir")
            .to_owned();

        println!(
            "Starting TorService with Datadir {} SocksPort {}",
            dir_str, socks_port
        );
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
            let message = match e.downcast::<String>() {
                Ok(msg) => *msg,
                Err(_) => String::from("Unknown panic"),
            };
            Box::into_raw(Box::new(BoxedResult {
                result: None,
                message: ResultMessage::Error(CString::new(message).unwrap().into_raw()),
            }))
        }
    }
}
#[no_mangle]
///# Safety
/// Get the status of a OwnedTorService
pub unsafe extern "C" fn get_status_of_owned_TorService(
    owned_client: *mut OwnedTorService,
) -> *mut c_char {
    assert!(!owned_client.is_null());
    let owned = &mut *owned_client;
    let node_status = owned.get_status();
    match node_status {
        Ok(status) => {
            let status_string = serde_json::to_string(&status).unwrap();
            println!("status is {}", status_string);
            CString::new(status_string).unwrap().into_raw()
        }
        Err(e) => {
            let message = match e.downcast::<String>() {
                Ok(msg) => msg,
                Err(_) => String::from("Unknown error"),
            };
            CString::new(message).unwrap().into_raw()
        }
    }
}
#[no_mangle]
///# Safety
/// Get the status of a OwnedTorService
pub unsafe extern "C" fn msg_over_tcp(
    owned_client: *mut OwnedTorService,
    target: *const c_char,
    msg: *const c_char,
    ffi_callback: fn(CString),
) {
    assert!(!owned_client.is_null());
    let owned = &mut *owned_client;

    let msg_str: String = unsafe { CStr::from_ptr(msg) }
        .to_str()
        .expect("Could not get str from msg")
        .into();

    let target_str: String = unsafe { CStr::from_ptr(target) }
        .to_str()
        .expect("Could not get str from target")
        .into();

    owned
        .msg_over_tcp(
            MsgOverTcp {
                target: target_str,
                msg: msg_str,
            },
            Some(move |reply: String| ffi_callback(CString::new(reply).unwrap())),
        )
        .unwrap();
    // match msg_reply {
    //     Ok(reply) => {
    //         let status_string = serde_json::to_string(&reply).unwrap();
    //         println!("reply is {}", status_string);
    //         CString::new(status_string).unwrap().into_raw()
    //     }
    //     Err(e) => {
    //         let message = match e.downcast::<String>() {
    //             Ok(msg) => msg,
    //             Err(_) => String::from("Unknown error"),
    //         };
    //         CString::new(message).unwrap().into_raw()
    //     }
    // }
}

#[no_mangle]
///# Safety
/// Destroy a cstr
pub unsafe extern "C" fn destroy_cstr(c_str: *mut c_char) {
    assert!(!c_str.is_null());
    let _ = Box::from_raw(c_str);
}

//
#[no_mangle]
///# Safety
/// Destroy and release ownedTorBox which will shut down owned connection and shutdown daemon
pub unsafe extern "C" fn shutdown_owned_TorService(owned_client: *mut OwnedTorService) {
    assert!(!owned_client.is_null());
    let mut owned: Box<OwnedTorService> = Box::from_raw(owned_client);
    owned.shutdown();
}
