#[cfg(target_os = "android")]
use android_logger::{Config, FilterBuilder};
use log::debug;
use once_cell::sync::OnceCell;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_uchar, c_ulong, c_ushort};
use std::sync::Mutex;

static INITIALIZED: OnceCell<bool> = OnceCell::new();

use crate::{
    ensure_runtime, OwnedTorService, OwnedTorServiceBootstrapPhase, TorHiddenServiceParam,
    TorServiceParam,
};

// Global state management for the Tor service
static TOR_SERVICE: OnceCell<Mutex<Option<OwnedTorService>>> = OnceCell::new();

fn ensure_tor_service() -> &'static Mutex<Option<OwnedTorService>> {
    TOR_SERVICE.get_or_init(|| Mutex::new(None))
}

// Add this function to initialize logging
pub fn init_logging() {
    #[cfg(target_os = "android")]
    android_logger::init_once(
        Config::default()
            .with_max_level(log::LevelFilter::Debug)
            .with_tag("RustTor") // This will be your logcat tag
            .with_filter(FilterBuilder::new().parse("debug,tor=debug").build()),
    );
}

// C-compatible structs with primitive types only
#[repr(C)]
pub struct HiddenServiceResponse {
    pub is_success: bool,
    pub onion_address: *mut c_char,
    pub control: *mut c_char,
}

#[repr(C)]
pub struct StartTorResponse {
    pub is_success: bool,
    pub onion_address: *mut c_char,
    pub control: *mut c_char,
    pub error_message: *mut c_char,
}

// Helper to create a C string from Rust string
fn to_c_string(s: String) -> *mut c_char {
    let c_str = CString::new(s).unwrap_or_else(|_| CString::new("").unwrap());
    c_str.into_raw()
}

// Helper to create an empty C string
fn empty_c_string() -> *mut c_char {
    let c_str = CString::new("").unwrap();
    c_str.into_raw()
}

// Helper function to safely convert C string to Rust string
fn from_c_str(s: *const c_char) -> String {
    if s.is_null() {
        return String::new();
    }

    unsafe { CStr::from_ptr(s).to_string_lossy().into_owned() }
}

// Export functions with C ABI
#[no_mangle]
pub extern "C" fn initialize_tor_library() -> bool {
    if INITIALIZED.get().is_some() {
        return true;
    }

    #[cfg(target_os = "android")]
    init_logging();

    // Initialize runtime
    let _ = ensure_runtime();

    // Initialize TOR_SERVICE
    let _ = ensure_tor_service();

    match INITIALIZED.set(true) {
        Ok(_) => true,
        Err(_) => false,
    }
}

#[no_mangle]
pub extern "C" fn init_tor_service(
    socks_port: c_ushort,
    data_dir: *const c_char,
    timeout_ms: c_ulong,
) -> bool {
    if INITIALIZED.get().is_none() {
        return false;
    }

    let data_dir_str = from_c_str(data_dir);

    debug!("Rust FFI: Initializing Tor service with parameters: socks_port={}, data_dir={}, timeout_ms={}", 
           socks_port, data_dir_str, timeout_ms);

    let param = TorServiceParam {
        socks_port: Some(socks_port as u16),
        data_dir: data_dir_str,
        bootstrap_timeout_ms: Some(timeout_ms as u64),
    };

    debug!(
        "Rust FFI: Initializing Tor service with parameters: {:?}",
        param
    );

    match OwnedTorService::new(param) {
        Ok(service) => {
            *ensure_tor_service().lock().unwrap() = Some(service);
            debug!("Rust FFI: Tor service initialized!");
            true
        }
        Err(e) => {
            debug!("Rust FFI: Error initializing Tor service! {:?}", e);
            false
        }
    }
}

#[no_mangle]
pub extern "C" fn create_hidden_service(
    port: c_ushort,
    target_port: c_ushort,
    key_data: *const c_uchar,
    has_key: bool,
) -> HiddenServiceResponse {
    let mut service_guard = ensure_tor_service().lock().unwrap();

    debug!(
        "Rust FFI: Creating hidden service with parameters: port={}, target_port={}, has_key={}",
        port, target_port, has_key
    );

    if let Some(service) = service_guard.as_mut() {
        let mut key_bytes = [0u8; 64];
        if has_key && !key_data.is_null() {
            unsafe {
                std::ptr::copy_nonoverlapping(key_data, key_bytes.as_mut_ptr(), 64);
            }
        }

        let param = TorHiddenServiceParam {
            to_port: target_port as u16,
            hs_port: port as u16,
            secret_key: if has_key { Some(key_bytes) } else { None },
        };

        debug!(
            "Rust FFI: Creating hidden service with parameters: {:?} and control port {} and control host {}",
            param.to_port, service.control_port.split(":").last().unwrap(), service.control_port.split(":").next().unwrap()
        );

        match service.create_hidden_service(param) {
            Ok(result) => {
                debug!("Rust FFI: Hidden service created {} ", result.onion_url);
                HiddenServiceResponse {
                    is_success: true,
                    onion_address: to_c_string(result.onion_url.to_string()),
                    control: to_c_string(service.control_port.trim().into()),
                }
            }
            Err(e) => {
                debug!("Rust FFI: Error creating hidden service {:?}", e);
                HiddenServiceResponse {
                    is_success: false,
                    onion_address: empty_c_string(),
                    control: empty_c_string(),
                }
            }
        }
    } else {
        debug!("Rust FFI: No service created");
        HiddenServiceResponse {
            is_success: false,
            onion_address: empty_c_string(),
            control: empty_c_string(),
        }
    }
}

#[no_mangle]
pub extern "C" fn start_tor_if_not_running(
    data_dir: *const c_char,
    key_data: *const c_uchar,
    has_key: bool,
    socks_port: c_ushort,
    target_port: c_ushort,
    timeout_ms: c_ulong,
) -> StartTorResponse {
    // First initialize library if needed
    if !initialize_tor_library() {
        return StartTorResponse {
            is_success: false,
            onion_address: to_c_string(String::new()),
            control: to_c_string(String::new()),
            error_message: to_c_string("Failed to initialize Tor library".to_string()),
        };
    }

    // Check current service status
    let status = get_service_status();

    // If the service is already ready (status = 1) or in progress (status = 0),
    // we can attempt to create a hidden service without re-initializing
    if status == 2 {
        // Only initialize if status indicates error or not initialized
        debug!(
            "Rust FFI: Tor service needs initialization. Status: {}",
            status
        );

        // Initialize Tor service
        if !init_tor_service(socks_port, data_dir, timeout_ms) {
            return StartTorResponse {
                is_success: false,
                onion_address: empty_c_string(),
                control: empty_c_string(),
                error_message: to_c_string("Failed to initialize Tor service".to_string()),
            };
        }
    } else {
        debug!(
            "Rust FFI: Tor service already initialized. Status: {}",
            status
        );
    }

    // Create hidden service
    let hs_response = create_hidden_service(socks_port, target_port, key_data, has_key);

    // Create a response with simple types only
    StartTorResponse {
        is_success: hs_response.is_success,
        onion_address: if hs_response.is_success {
            hs_response.onion_address
        } else {
            empty_c_string()
        },
        control: if hs_response.is_success {
            hs_response.control
        } else {
            empty_c_string()
        },
        error_message: if hs_response.is_success {
            empty_c_string()
        } else {
            to_c_string("Failed to create hidden service".to_string())
        },
    }
}

#[no_mangle]
pub extern "C" fn get_service_status() -> c_int {
    let service_guard = ensure_tor_service().lock().unwrap();

    match &*service_guard {
        Some(service) => match service.get_status() {
            Ok(OwnedTorServiceBootstrapPhase::Done) => 1,
            Ok(_) => 0,
            Err(_) => 2,
        },
        None => 2,
    }
}

#[no_mangle]
pub extern "C" fn delete_hidden_service(address: *const c_char) -> bool {
    let mut service_guard = ensure_tor_service().lock().unwrap();
    let address_str = from_c_str(address);

    if let Some(service) = service_guard.as_mut() {
        service.delete_hidden_service(address_str).is_ok()
    } else {
        false
    }
}

#[no_mangle]
pub extern "C" fn shutdown_service() -> bool {
    let mut service_guard = ensure_tor_service().lock().unwrap();

    if let Some(mut service) = service_guard.take() {
        service.shutdown().is_ok()
    } else {
        false
    }
}

// Clean up allocated C strings
#[no_mangle]
pub extern "C" fn free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s);
        }
    }
}
