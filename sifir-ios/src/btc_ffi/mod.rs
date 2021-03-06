#[cfg(feature = "btc_wallet")]
use crate::tor_ffi::{BoxedResult, ResultMessage};
use btc::DerivedBip39Xprvs;
use libc::{c_char, c_void};
use std::ffi::{CStr, CString};
use std::panic::catch_unwind;

#[no_mangle]
pub extern "C" fn new_wallet_descriptors(
    network: *const c_char,
    derive_path: *const c_char,
    password: *const c_char,
) -> *mut BoxedResult<DerivedBip39Xprvs> {
    match catch_unwind(|| {
        assert!(!network.is_null());
        assert!(!derive_path.is_null());
        assert!(!password.is_null());

        let network_str = unsafe { CStr::from_ptr(network) }
            .to_str()
            .expect("Could not get str from data_dir");
        let derive_path_str = unsafe { CStr::from_ptr(derive_path) }
            .to_str()
            .expect("Could not get str from data_dir");
        let password_srr = unsafe { CStr::from_ptr(password) }
            .to_str()
            .expect("Could not get str from data_dir");
        let network = match network_str {
            "testnet" => Ok(Network::Testnet),
            "mainnet" => Ok(Network::Bitcoin),
            _ => Err("Invalid network passed"),
        }
        .unwrap();

        DerivedBip39Xprvs::new(
            derive_path_str.into_derivation_path().unwrap(),
            network,
            2,
            Some(String::from(password_srr)),
            None,
        )
        .unwrap()
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
