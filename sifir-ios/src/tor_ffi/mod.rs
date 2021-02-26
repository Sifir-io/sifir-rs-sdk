use libc::{c_char, c_void};
use std::ffi::{CStr, CString};
use std::panic::catch_unwind;
use std::time::Duration;
use tor::{
    tcp_stream::{DataObserver, TcpSocksStream},
    OwnedTorService, TorServiceParam,
};

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
            "Starting TorService with in data dir {} SocksPort {}",
            dir_str, socks_port
        );
        let param = TorServiceParam {
            socks_port: Some(socks_port),
            data_dir: dir_str,
        };
        OwnedTorService::new(param)?
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
/// Start a proxied TcpStream
pub unsafe extern "C" fn tcp_stream_start(
    target: *const c_char,
    proxy: *const c_char,
    timeout_ms: u64,
) -> *mut BoxedResult<TcpSocksStream> {
    match catch_unwind(|| {
        assert!(!target.is_null());
        assert!(!proxy.is_null());
        let proxy_str: String = unsafe { CStr::from_ptr(proxy) }
            .to_str()
            .expect("Could not get str from proxy")
            .into();

        let target_str: String = unsafe { CStr::from_ptr(target) }
            .to_str()
            .expect("Could not get str from target")
            .into();

        TcpSocksStream::new_timeout(target_str, proxy_str, timeout_ms).unwrap()
    }) {
        Ok(stream) => Box::into_raw(Box::new(BoxedResult {
            result: Some(Box::new(stream)),
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

#[repr(C)]
pub struct Observer {
    context: *mut c_void,
    on_success: extern "C" fn(*mut c_char, *const c_void),
    on_err: extern "C" fn(*mut c_char, *const c_void),
}

unsafe impl Send for Observer {}

impl DataObserver for Observer {
    fn on_data(&self, data: String) {
        (self.on_success)(CString::new(data).unwrap().into_raw(), self.context);
    }
    fn on_error(&self, data: String) {
        (self.on_err)(CString::new(data).unwrap().into_raw(), self.context);
    }
}
#[no_mangle]
///# Safety
/// Send a Message over a tcpStream
pub unsafe extern "C" fn tcp_stream_on_data(
    stream: *mut TcpSocksStream,
    observer: Observer,
) -> *mut ResultMessage {
    match catch_unwind(|| {
        assert!(!stream.is_null());
        let stream = &mut *stream;
        stream.on_data(observer).unwrap()
    }) {
        Ok(_) => Box::into_raw(Box::new(ResultMessage::Success)),
        Err(e) => {
            let message = match e.downcast::<String>() {
                Ok(msg) => *msg,
                Err(_) => String::from("Unknown panic"),
            };
            Box::into_raw(Box::new(ResultMessage::Error(
                CString::new(message).unwrap().into_raw(),
            )))
        }
    }
}
#[no_mangle]
///# Safety
/// Send a Message over a tcpStream
pub unsafe extern "C" fn tcp_stream_send_msg(
    stream: *mut TcpSocksStream,
    msg: *const c_char,
    timeout: u64,
) -> *mut ResultMessage {
    match catch_unwind(|| {
        assert!(!stream.is_null());
        assert!(!msg.is_null());
        let stream = &mut *stream;
        let msg_str: String = unsafe { CStr::from_ptr(msg) }
            .to_str()
            .expect("Could not get str from proxy")
            .into();
        stream
            .send_data(msg_str, Some(Duration::new(timeout, 0)))
            .unwrap()
    }) {
        Ok(_) => Box::into_raw(Box::new(ResultMessage::Success)),
        Err(e) => {
            let message = match e.downcast::<String>() {
                Ok(msg) => *msg,
                Err(_) => String::from("Unknown panic"),
            };
            Box::into_raw(Box::new(ResultMessage::Error(
                CString::new(message).unwrap().into_raw(),
            )))
        }
    }
}
// FIXME here on_data interface

#[no_mangle]
///# Safety
/// Destroy and release TcpSocksStream which will drop the connection
pub unsafe extern "C" fn tcp_stream_destroy(stream: *mut TcpSocksStream) {
    assert!(!stream.is_null());
    let _ = Box::from_raw(stream);
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
