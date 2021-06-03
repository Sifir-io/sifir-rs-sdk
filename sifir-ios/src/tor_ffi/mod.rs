use libc::{c_char, c_void};
use serde_json::json;
use std::ffi::{CStr, CString};
use std::ops::{Deref, DerefMut};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::time::Duration;
use tor::{
    hidden_service::{HiddenServiceDataHandler, HiddenServiceHandler},
    tcp_stream::{DataObserver, TcpSocksStream},
    OwnedTorService, TorHiddenService, TorHiddenServiceParam, TorServiceParam,
};
use logger;

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
/// Starts env logger
pub extern "C" fn start_logger() {
    logger::Logger::new();
}

#[no_mangle]
pub extern "C" fn get_owned_TorService(
    data_dir: *const c_char,
    socks_port: u16,
    bootstrap_timeout_ms: u64,
) -> *mut BoxedResult<OwnedTorService> {
    match catch_unwind(|| {
        assert!(!data_dir.is_null());
        let dir_str: String = unsafe { CStr::from_ptr(data_dir) }
            .to_str()
            .expect("Could not get str from data_dir")
            .to_owned();

        println!(
            "Starting TorService with in data dir '{}' SocksPort '{}' with bootstrap timeout '{}' ... ",
            dir_str, socks_port,bootstrap_timeout_ms
        );
        let param = TorServiceParam {
            socks_port: Some(socks_port),
            data_dir: dir_str,
            bootstrap_timeout_ms: Some(bootstrap_timeout_ms),
        };
        OwnedTorService::new(param).unwrap()
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
pub extern "C" fn get_status_of_owned_TorService(
    owned_client: *mut OwnedTorService,
) -> *mut c_char {
    assert!(!owned_client.is_null());
    let owned = unsafe { &mut *owned_client };
    let node_status = owned.get_status();
    match node_status {
        Ok(status) => {
            let status_string = serde_json::to_string(&status).unwrap();
            CString::new(status_string).unwrap().into_raw()
        }
        Err(e) => {
            let message: String = format!("Error {:?}", e);
            CString::new(message).unwrap().into_raw()
        }
    }
}
#[no_mangle]
///# Safety
/// Start a proxied TcpStream
pub extern "C" fn tcp_stream_start(
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
unsafe impl Sync for Observer {}

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
pub extern "C" fn tcp_stream_on_data(
    stream_ptr: *mut TcpSocksStream,
    observer: Observer,
) -> *mut ResultMessage {
    assert!(!stream_ptr.is_null());
    match {
        let stream = unsafe { &mut *stream_ptr };
        if let Err(e) = stream.set_data_handler(observer) {
            Err(e)
        } else {
            stream.read_line_async()
        }
    } {
        Ok(_) => Box::into_raw(Box::new(ResultMessage::Success)),
        Err(e) => {
            let message = format!("{:?}", e);
            Box::into_raw(Box::new(ResultMessage::Error(
                CString::new(message).unwrap().into_raw(),
            )))
        }
    }
}
#[no_mangle]
///# Safety
/// Send a Message over a tcpStream
pub extern "C" fn tcp_stream_send_msg(
    stream_ptr: *mut TcpSocksStream,
    msg: *const c_char,
    timeout: u64,
) -> *mut ResultMessage {
    assert!(!stream_ptr.is_null());
    assert!(!msg.is_null());
    match {
        let mut stream = unsafe { &mut *stream_ptr };
        let msg_str: String = unsafe { CStr::from_ptr(msg) }
            .to_str()
            .expect("Could not get str from proxy")
            .into();
        stream.send_data(msg_str, Some(Duration::new(timeout, 0)))
    } {
        Ok(_) => Box::into_raw(Box::new(ResultMessage::Success)),
        Err(e) => {
            let message = format!("{:?}", e);
            Box::into_raw(Box::new(ResultMessage::Error(
                CString::new(message).unwrap().into_raw(),
            )))
        }
    }
}
#[no_mangle]
///# Safety
/// Creates a Hidden service returning it's secret/public key
pub extern "C" fn create_hidden_service(
    owned_client: *mut OwnedTorService,
    dst_port: u16,
    hs_port: u16,
) -> *mut BoxedResult<*mut c_char> {
    assert!(!owned_client.is_null());
    let owned = unsafe { &mut *owned_client };
    let hidden_service = owned.create_hidden_service(TorHiddenServiceParam {
        to_port: dst_port,
        hs_port,
        secret_key: None,
    });
    match hidden_service {
        Ok(TorHiddenService {
            onion_url,
            secret_key,
        }) => {
            let json_payload = json!({ "onion_url": onion_url.to_string(), "secret_key": base64::encode(secret_key) });
            Box::into_raw(Box::new(BoxedResult {
                result: Some(Box::new(
                    CString::new(json_payload.to_string()).unwrap().into_raw(),
                )),
                message: ResultMessage::Success,
            }))
        }
        Err(e) => {
            let message = format!("{:#?}", e);
            Box::into_raw(Box::new(BoxedResult {
                result: None,
                message: ResultMessage::Error(CString::new(message).unwrap().into_raw()),
            }))
        }
    }
}
#[no_mangle]
///# Safety
/// Starts an HTTP request server on dst_port calling the observer with data
pub extern "C" fn start_http_hidden_service_handler(
    dst_port: u16,
    observer: Observer,
) -> *mut BoxedResult<HiddenServiceHandler> {
    match HiddenServiceHandler::new(dst_port) {
        Ok(mut lsner) => match lsner.set_data_handler(observer) {
            Ok(_) => {
                let _ = lsner.start_http_listener();
                Box::into_raw(Box::new(BoxedResult {
                    result: Some(Box::new(lsner)),
                    message: ResultMessage::Success,
                }))
            }
            Err(e) => {
                let message = format!("error setting data handler: {:#?}", e);
                Box::into_raw(Box::new(BoxedResult {
                    result: None,
                    message: ResultMessage::Error(CString::new(message).unwrap().into_raw()),
                }))
            }
        },
        Err(e) => {
            let message = format!("{:#?}", e);
            Box::into_raw(Box::new(BoxedResult {
                result: None,
                message: ResultMessage::Error(CString::new(message).unwrap().into_raw()),
            }))
        }
    }
}

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
    let _ = owned.shutdown();
}
#[no_mangle]
///# Safety
/// Destroy and release HiddenServiceHandler
pub unsafe extern "C" fn destroy_hidden_service_handler(hs_handler: *mut HiddenServiceHandler) {
    assert!(!hs_handler.is_null());
    let _: Box<HiddenServiceHandler> = Box::from_raw(hs_handler);
}
