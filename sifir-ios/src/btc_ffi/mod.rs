use crate::util::*;
use btc::*;
use libc::{c_char, c_void};
use std::ffi::{CStr, CString};
use std::panic::catch_unwind;

macro_rules! unwind_into_boxed_result {
    ($e:expr) => {
        match catch_unwind(|| $e) {
            Ok(x) => Box::into_raw(Box::new(BoxedResult {
                result: Some(Box::new(x)),
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
    };
}

macro_rules! required_str_from_cchar_ptr {
    ($e:expr) => {
        unsafe { CStr::from_ptr($e) }
            .to_str()
            .expect("Could not get str!");
    };
}

#[no_mangle]
pub extern "C" fn derive_xprvs(
    network: *const c_char,
    derive_path: *const c_char,
    password: *const c_char,
    seed_phrase: *const c_char,
    num_child: usize,
) -> *mut BoxedResult<*mut c_char> {
    unwind_into_boxed_result!({
        assert!(!network.is_null());
        assert!(!derive_path.is_null());
        assert!(!password.is_null());
        assert!(!seed_phrase.is_null());

        let network_str = unsafe { CStr::from_ptr(network) }
            .to_str()
            .expect("Could not get str from data_dir");
        let derive_path_str = unsafe { CStr::from_ptr(derive_path) }
            .to_str()
            .expect("Could not get str from data_dir");
        let password_srr = unsafe { CStr::from_ptr(password) }
            .to_str()
            .expect("Could not get str from data_dir");
        let mnemonic = unsafe { CStr::from_ptr(seed_phrase) }
            .to_str()
            .expect("Could not get str from data_dir");

        let network = match network_str {
            "testnet" => Ok(Network::Testnet),
            "mainnet" => Ok(Network::Bitcoin),
            "bitcoin" => Ok(Network::Bitcoin),
            _ => Err("Invalid network passed"),
        }
        .unwrap();

        let num_child = match num_child {
            x if x >= 2 => x,
            _ => 2,
        };

        let wallet_desc = DerivedBip39Xprvs::new(
            derive_path_str.into_derivation_path().unwrap(),
            network,
            num_child,
            Some(String::from(password_srr)),
            match mnemonic.len() {
                x if x > 0 => Some(String::from(mnemonic)),
                _ => None,
            },
        )
        .unwrap();

        let json = serde_json::to_string(&wallet_desc).unwrap();
        CString::new(json).unwrap().into_raw()
    })
}

#[no_mangle]
pub extern "C" fn descriptors_from_xprvs_wpaths_vec(
    vec_xprvs_with_paths_json: *const c_char,
    network: *const c_char,
) -> *mut BoxedResult<*mut c_char> {
    unwind_into_boxed_result!({
        let xprvspaths_str = required_str_from_cchar_ptr!(vec_xprvs_with_paths_json);
        let network_str = required_str_from_cchar_ptr!(network);
        let network = match network_str {
            "testnet" => Ok(Network::Testnet),
            "mainnet" => Ok(Network::Bitcoin),
            "bitcoin" => Ok(Network::Bitcoin),
            _ => Err("Invalid network passed"),
        }
        .unwrap();
        let x_prvs_with_path: Vec<XprvsWithPaths> = serde_json::from_str(xprvspaths_str).unwrap();
        let wallet_descriptors: WalletDescriptors = (x_prvs_with_path, network).into();
        let json = serde_json::to_string(&wallet_descriptors).unwrap();
        CString::new(json).unwrap().into_raw()
    })
}

// FIXME HERE
// 1. how to persist and TEST !
#[no_mangle]
pub extern "C" fn electrum_wallet_from_wallet_cfg(
    wallet_cfg_json: *const c_char,
) -> *mut BoxedResult<ElectrumSledWallet> {
    unwind_into_boxed_result!({
        let wallet_cfg_str = required_str_from_cchar_ptr!(wallet_cfg_json);
        let wallet_cfg: WalletCfg = serde_json::from_str(wallet_cfg_str).unwrap();
        let wallet: ElectrumSledWallet = wallet_cfg.into();
        wallet
    })
}

#[no_mangle]
///# Safety
/// Destroy a cstr
pub unsafe extern "C" fn destroy_cstr(c_str: *mut c_char) {
    assert!(!c_str.is_null());
    let _ = Box::from_raw(c_str);
}
