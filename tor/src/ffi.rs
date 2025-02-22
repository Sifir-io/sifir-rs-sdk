#[cfg(target_os = "android")]
use android_logger::{Config, FilterBuilder};
use log::{info, LevelFilter};

// Add this function to initialize logging
pub fn init_logging() {
    #[cfg(target_os = "android")]
    android_logger::init_once(
        Config::default()
            .with_max_level(LevelFilter::Debug)
            .with_tag("RustTor") // This will be your logcat tag
            .with_filter(FilterBuilder::new().parse("debug,tor=debug").build()),
    );
}

#[cxx::bridge(namespace = "tor")]
mod ffi {
    extern "Rust" {
        // Initialize Tor service with direct parameters
        fn init_tor_service(socks_port: u16, data_dir: &str, timeout_ms: u64) -> bool;

        // Create hidden service with direct parameters
        fn create_hidden_service(
            port: u16,
            target_port: u16,
            key_data: &[u8; 64],
            has_key: bool,
        ) -> String;

        // Simple status check returning integer for state
        fn get_service_status() -> i32; // 0 = not ready, 1 = ready, 2 = error

        // Simple deletion with boolean result
        fn delete_hidden_service(address: &str) -> bool;

        // Simple shutdown with boolean result
        fn shutdown_service() -> bool;
    }
}

use std::sync::Mutex;

// Global state management for the Tor service
use lazy_static::lazy_static;
use logger::log::debug;

use crate::{
    OwnedTorService, OwnedTorServiceBootstrapPhase, TorHiddenServiceParam, TorServiceParam,
};

lazy_static! {
    static ref TOR_SERVICE: Mutex<Option<OwnedTorService>> = Mutex::new(None);
}

// Implementation of the simplified interface
fn init_tor_service(socks_port: u16, data_dir: &str, timeout_ms: u64) -> bool {
    #[cfg(target_os = "android")]
    init_logging();

    info!("Rust FFI: Initializing Tor service with parameters: socks_port={}, data_dir={}, timeout_ms={}", socks_port, data_dir, timeout_ms);

    let param = TorServiceParam {
        socks_port: Some(socks_port),
        data_dir: data_dir.to_string(),
        bootstrap_timeout_ms: Some(timeout_ms),
    };

    info!(
        "Rust FFI: Initializing Tor service with parameters: {:?}",
        param
    );

    match OwnedTorService::new(param) {
        Ok(service) => {
            *TOR_SERVICE.lock().unwrap() = Some(service);

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
) -> String {
    let mut service_guard = TOR_SERVICE.lock().unwrap();

    info!(
        "Rust FFI: Creating hidden service with parameters: port={}, target_port={}, key_data={:?}, has_key={}",
        port, target_port, key_data, has_key
    );

    if let Some(service) = service_guard.as_mut() {
        let param = TorHiddenServiceParam {
            to_port: target_port,
            hs_port: port,
            secret_key: if has_key { Some(*key_data) } else { None },
        };

        debug!(
            "Rust FFI: Creating hidden service with parameters: {:?}",
            param.to_port
        );

        match service.create_hidden_service(param) {
            Ok(result) => result.onion_url.to_string(),
            Err(_) => String::new(),
        }
    } else {
        String::new()
    }
}

fn get_service_status() -> i32 {
    let service_guard = TOR_SERVICE.lock().unwrap();

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
    let mut service_guard = TOR_SERVICE.lock().unwrap();

    if let Some(service) = service_guard.as_mut() {
        service.delete_hidden_service(address.to_string()).is_ok()
    } else {
        false
    }
}

fn shutdown_service() -> bool {
    let mut service_guard = TOR_SERVICE.lock().unwrap();

    if let Some(mut service) = service_guard.take() {
        service.shutdown().is_ok()
    } else {
        false
    }
}
