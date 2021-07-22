use crate::util::*;
use btc::*;
use libc::{c_char, c_void};
use serde_json::json;
use std::ffi::{CStr, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};

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
            .expect("Could not get str from network");
        let derive_path_str = unsafe { CStr::from_ptr(derive_path) }
            .to_str()
            .expect("Could not get str from derive_path");
        let password_srr = unsafe { CStr::from_ptr(password) }
            .to_str()
            .expect("Could not get str from password_str");
        let mnemonic = unsafe { CStr::from_ptr(seed_phrase) }
            .to_str()
            .expect("Could not get str from seed_phrase");

        let network = match network_str {
            "testnet" => Ok(Network::Testnet),
            "mainnet" => Ok(Network::Bitcoin),
            "bitcoin" => Ok(Network::Bitcoin),
            _ => Err("Invalid network passed"),
        }
        .unwrap();

        let num_child: u32 = match num_child {
            x if x >= 2 => x as u32,
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
pub extern "C" fn get_electrum_wallet_balance(
    electrum_wallet: *mut ElectrumSledWallet,
) -> *mut BoxedResult<u64> {
    let wallet = unsafe { &mut *electrum_wallet };
    let matcher = AssertUnwindSafe(wallet);
    unwind_into_boxed_result!({ matcher.get_balance().unwrap() })
}

#[no_mangle]
pub extern "C" fn get_electrum_wallet_new_address(
    electrum_wallet: *mut ElectrumSledWallet,
) -> *mut BoxedResult<*mut c_char> {
    let wallet = unsafe { &mut *electrum_wallet };
    let matcher = AssertUnwindSafe(wallet);
    unwind_into_boxed_result!({
        let address = matcher.get_address(AddressIndex::New).unwrap();
        CString::new(format!("{}", address)).unwrap().into_raw()
    })
}

#[no_mangle]
pub extern "C" fn sync_electrum_wallet(
    electrum_wallet: *mut ElectrumSledWallet,
    max_address_count: u32,
) -> *mut BoxedResult<bool> {
    let wallet = unsafe { &mut *electrum_wallet };
    let matcher = AssertUnwindSafe(wallet);
    unwind_into_boxed_result!({
        struct SifirWallet {};
        impl Progress for SifirWallet {
            fn update(&self, progress: f32, message: Option<String>) -> Result<(), bdk::Error> {
                println!(
                    "ios ffi sync progress is {} and message {:?}, TODO THIS TO OBSERVER",
                    progress, message
                );
                Ok(())
            }
        };
        let _ = matcher
            .sync(SifirWallet {}, Some(max_address_count))
            .unwrap();
        true
    })
}

/// Generates a finalized txn from CreateTxn json
#[no_mangle]
pub extern "C" fn create_tx(
    wallet: *mut ElectrumSledWallet,
    tx: *const c_char,
) -> *mut BoxedResult<*mut c_char> {
    let wallet = unsafe { &mut *wallet };
    let matcher = AssertUnwindSafe(wallet);
    unwind_into_boxed_result!({
        let txn_str = required_str_from_cchar_ptr!(tx);
        let create_txn: CreateTx = serde_json::from_str(txn_str).unwrap();
        let (pp, txn) = create_txn.into_wallet_txn(&matcher).unwrap();
        let txn_json = json!({"partiallySignedPsbt": pp, "txnDetails" : txn});
        CString::new(txn_json.to_string()).unwrap().into_raw()
    })
}

#[no_mangle]
pub unsafe extern "C" fn drop_wallet(wallet: *mut ElectrumSledWallet) {
    assert!(!wallet.is_null());
    let _: Box<ElectrumSledWallet> = Box::from_raw(wallet);
}
