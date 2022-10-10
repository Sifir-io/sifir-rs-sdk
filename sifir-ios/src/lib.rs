mod util;

use libc::{c_char, c_void};
use log::*;
use logger::{log, Logger};
use reqwest;
use std::ffi::{CStr, CString};

#[cfg(feature = "tor_daemon")]
mod tor_ffi;
use tor::tcp_stream::DataObserver;
use tor::RUNTIME;
use tor_ffi::Observer;

#[cfg(feature = "btc_wallet")]
mod btc_ffi;

///# Safety
/// Init the platform's logger
#[no_mangle]
pub extern "C" fn init_logger() {
    Logger::new();
}
pub fn get_proxied_client(socks_port: u16) -> Result<reqwest::Client, reqwest::Error> {
    let proxy = reqwest::Proxy::all(
        reqwest::Url::parse(format!("socks5h://127.0.0.1:{}", socks_port).as_str()).unwrap(),
    )
    .unwrap();
    reqwest::Client::builder().proxy(proxy).build()
}
#[no_mangle]
///# Safety
pub extern "C" fn get(socks_port: u16, url: *const c_char, observer: Observer) {
    let client = get_proxied_client(socks_port).unwrap();
    let get_url: String = unsafe { CStr::from_ptr(url) }
        .to_str()
        .expect("Could not get str from url")
        .to_owned();
    (*RUNTIME).lock().unwrap().spawn(async move {
        let resp = client.get(get_url).send().await;
        match resp {
            Ok(r) => observer.on_data(r.text().await.unwrap()),
            Err(e) => {
                error!("Error processing request {}", e);
                observer.on_error(e.to_string());
            }
        }
    });
}
