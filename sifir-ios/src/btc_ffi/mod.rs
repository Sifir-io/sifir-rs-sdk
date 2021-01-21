use crate::tor_ffi::{BoxedResult, ResultMessage};
use btc::{generate_wallet_descriptors, Network, WalletCfg, WalletDescriptors};
use libc::{c_char, c_void};
use std::ffi::{CStr, CString};
use std::panic::catch_unwind;

#[no_mangle]
pub extern "C" fn generate_descriptor(
    network: *const c_char,
) -> *mut BoxedResult<WalletDescriptors> {
    match catch_unwind(|| {
        assert!(!network.is_null());
        let network_str = unsafe { CStr::from_ptr(network) }
            .to_str()
            .expect("Could not get str from data_dir");

        let network = match network_str {
            "testnet" => Ok(Network::Testnet),
            "mainnet" => Ok(Network::Bitcoin),
            _ => Err("Invalid network passed"),
        }.unwrap();
        generate_wallet_descriptors(network).unwrap()
    }) {
        Ok(descriptor) => Box::into_raw(Box::new(BoxedResult {
            result: Some(Box::new(descriptor)),
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
