#[cfg(target_os = "android")]
use android_logger::{Config, FilterBuilder};
use log::{debug, info};
use once_cell::sync::OnceCell;

static INITIALIZED: OnceCell<bool> = OnceCell::new();

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

#[cxx::bridge(namespace = "tor")]
mod ffi {
    // Simple output structs with primitive types only
    struct HiddenServiceResponse {
        is_success: bool,
        onion_address: String,
        control: String,
    }

    struct StartTorResponse {
        is_success: bool,
        onion_address: String,
        control: String,
        error_message: String,
    }

    extern "Rust" {
        fn initialize_tor_library() -> bool;
        fn init_tor_service(socks_port: u16, data_dir: &str, timeout_ms: u64) -> bool;
        fn create_hidden_service(
            port: u16,
            target_port: u16,
            key_data: &[u8; 64],
            has_key: bool,
        ) -> HiddenServiceResponse;
        fn start_tor_if_not_running(
            data_dir: &str,
            key_data: &[u8; 64],
            has_key: bool,
            socks_port: u16,
            target_port: u16,
            timeout_ms: u64,
        ) -> StartTorResponse;
        fn get_service_status() -> i32;
        fn delete_hidden_service(address: &str) -> bool;
        fn shutdown_service() -> bool;
    }
}

// Implementation for start_tor_if_not_running
fn start_tor_if_not_running(
    data_dir: &str,
    key_data: &[u8; 64],
    has_key: bool,
    socks_port: u16,
    target_port: u16,
    timeout_ms: u64,
) -> ffi::StartTorResponse {
    // First initialize library if needed
    if !initialize_tor_library() {
        return ffi::StartTorResponse {
            is_success: false,
            onion_address: String::new(),
            control: String::new(),
            error_message: "Failed to initialize Tor library".to_string(),
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
            return ffi::StartTorResponse {
                is_success: false,
                onion_address: String::new(),
                control: String::new(),
                error_message: "Failed to initialize Tor service".to_string(),
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
    ffi::StartTorResponse {
        is_success: hs_response.is_success,
        onion_address: hs_response.onion_address,
        control: hs_response.control.trim().into(),
        error_message: if hs_response.is_success {
            String::new()
        } else {
            "Failed to create hidden service".to_string()
        },
    }
}

use std::sync::Mutex;

use crate::{
    ensure_runtime, OwnedTorService, OwnedTorServiceBootstrapPhase, TorHiddenServiceParam,
    TorServiceParam,
};

// Global state management for the Tor service
static TOR_SERVICE: OnceCell<Mutex<Option<OwnedTorService>>> = OnceCell::new();

fn ensure_tor_service() -> &'static Mutex<Option<OwnedTorService>> {
    TOR_SERVICE.get_or_init(|| Mutex::new(None))
}

// We first initialize the run times.
// Then we check the global state
fn initialize_tor_library() -> bool {
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

// Implementation of the simplified interface
fn init_tor_service(socks_port: u16, data_dir: &str, timeout_ms: u64) -> bool {
    if INITIALIZED.get().is_none() {
        return false;
    }

    debug!("Rust FFI: Initializing Tor service with parameters: socks_port={}, data_dir={}, timeout_ms={}", socks_port, data_dir, timeout_ms);

    let param = TorServiceParam {
        socks_port: Some(socks_port),
        data_dir: data_dir.to_string(),
        bootstrap_timeout_ms: Some(timeout_ms),
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
        Err(_) => {
            debug!("Rust FFI: Error initializing Tor service!");
            false
        }
    }
}

fn create_hidden_service(
    port: u16,
    target_port: u16,
    key_data: &[u8; 64],
    has_key: bool,
) -> ffi::HiddenServiceResponse {
    let mut service_guard = ensure_tor_service().lock().unwrap();

    info!(
        "Rust FFI: Creating hidden service with parameters: port={}, target_port={}, has_key={}",
        port, target_port, has_key
    );

    if let Some(service) = service_guard.as_mut() {
        let param = TorHiddenServiceParam {
            to_port: target_port,
            hs_port: port,
            secret_key: if has_key { Some(*key_data) } else { None },
        };

        debug!(
            "Rust FFI: Creating hidden service with parameters: {:?} and control port {} and control host {}",
            param.to_port, service.control_port.split(":").last().unwrap(), service.control_port.split(":").next().unwrap()
        );

        match service.create_hidden_service(param) {
            Ok(result) => {
                debug!(
                    "Rust FFI: Hidden service created {:?} {}",
                    result, service.control_port
                );
                ffi::HiddenServiceResponse {
                    is_success: true,
                    onion_address: result.onion_url.to_string(),
                    control: service.control_port.trim().into(),
                }
            }
            Err(e) => {
                debug!("Rust FFI: Error creating hidden service {:?}", e);
                ffi::HiddenServiceResponse {
                    is_success: false,
                    onion_address: String::new(),
                    control: String::new(),
                }
            }
        }
    } else {
        debug!("Rust FFI: No service created");
        ffi::HiddenServiceResponse {
            is_success: false,
            onion_address: String::new(),
            control: String::new(),
        }
    }
}

fn get_service_status() -> i32 {
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

fn delete_hidden_service(address: &str) -> bool {
    let mut service_guard = ensure_tor_service().lock().unwrap();

    if let Some(service) = service_guard.as_mut() {
        service.delete_hidden_service(address.to_string()).is_ok()
    } else {
        false
    }
}

fn shutdown_service() -> bool {
    let mut service_guard = ensure_tor_service().lock().unwrap();

    if let Some(mut service) = service_guard.take() {
        service.shutdown().is_ok()
    } else {
        false
    }
}
