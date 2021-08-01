use crate::util::*;
use bdk::bitcoin::consensus::encode::{deserialize, serialize};
use bdk::bitcoin::util::psbt::PartiallySignedTransaction;
use bdk::SignOptions;
use btc::multi_sig::*;
use btc::*;
use libc::c_char;
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
pub extern "C" fn wallet_descriptors_from_any_descriptor_cfg(
    any_desc_cfg: *const c_char,
) -> *mut BoxedResult<*mut c_char> {
    unwind_into_boxed_result!({
        let any_desc_cfg_str = required_str_from_cchar_ptr!(any_desc_cfg);
        let wallet_descriptors: WalletDescriptors =
            serde_json::from_str::<AnyDescriptorCfg>(any_desc_cfg_str)
                .unwrap()
                .into();
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

// TODO  add index ? last, new , etcc...
#[no_mangle]
pub extern "C" fn get_electrum_wallet_new_address(
    electrum_wallet: *mut ElectrumSledWallet,
) -> *mut BoxedResult<*mut c_char> {
    let wallet = unsafe { &mut *electrum_wallet };
    let matcher = AssertUnwindSafe(wallet);
    unwind_into_boxed_result!({
        let address = matcher.get_address(AddressIndex::LastUnused).unwrap();
        CString::new(format!("{}", address)).unwrap().into_raw()
    })
}
// TODO  accept data observer
struct SifirWallet {}
impl Progress for SifirWallet {
    fn update(&self, progress: f32, message: Option<String>) -> Result<(), bdk::Error> {
        println!(
            "ios ffi sync progress is {} and message {:?}, TODO THIS TO OBSERVER",
            progress, message
        );
        Ok(())
    }
}

#[no_mangle]
pub extern "C" fn sync_electrum_wallet(
    electrum_wallet: *mut ElectrumSledWallet,
    max_address_count: u32,
) -> *mut BoxedResult<bool> {
    let wallet = unsafe { &mut *electrum_wallet };
    let matcher = AssertUnwindSafe(wallet);
    unwind_into_boxed_result!({
        let _ = matcher
            .sync(SifirWallet {}, Some(max_address_count))
            .unwrap();
        true
    })
}

/// Generates a finalized txn from CreateTxn json
/// returns json { psbt: base64, txnDetails: string }
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
        let (psbt, txn) = create_txn.into_wallet_txn(&matcher).unwrap();
        let txn_json = json!({"psbt": base64::encode(serialize(&psbt)), "txnDetails" : txn});
        CString::new(txn_json.to_string()).unwrap().into_raw()
    })
}


#[no_mangle]
pub extern "C" fn sign_psbt(
    electrum_wallet: *mut ElectrumSledWallet,
    psbt_base64: *const c_char,
) -> *mut BoxedResult<*mut c_char> {
    let psbt_str = required_str_from_cchar_ptr!(psbt_base64);
    let wallet = unsafe { &mut *electrum_wallet };
    let matcher = AssertUnwindSafe(wallet);

    unwind_into_boxed_result!({
        let mut psbt = deserialize(&base64::decode(&psbt_str).unwrap()).unwrap();
        let finished = matcher.sign(&mut psbt, SignOptions::default()).unwrap();
        let json =
            json!({"psbt" : base64::encode(serialize(&psbt)).to_string(), "finished": finished});
        CString::new(json.to_string()).unwrap().into_raw()
    })
}
#[no_mangle]
pub extern "C" fn broadcast_pbst(
    electrum_wallet: *mut ElectrumSledWallet,
    psbt_base64: *const c_char,
) -> *mut BoxedResult<*mut c_char> {
    let psbt_str = required_str_from_cchar_ptr!(psbt_base64);
    let wallet = unsafe { &mut *electrum_wallet };
    let matcher = AssertUnwindSafe(wallet);

    unwind_into_boxed_result!({
        let psbt =
            deserialize::<PartiallySignedTransaction>(&base64::decode(&psbt_str).unwrap()).unwrap();
        let txn_id = matcher.broadcast(psbt.extract_tx()).unwrap();
        CString::new(txn_id.to_string()).unwrap().into_raw()
    })
}

/// Convert XprvsWithPaths to XpubsWithPaths
#[no_mangle]
pub extern "C" fn xprvs_w_paths_to_xpubs_w_paths(
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
        let xpubs_with_paths: XpubsWithPaths = (
            serde_json::from_str::<XprvsWithPaths>(xprvspaths_str).unwrap(),
            network,
        )
            .into();
        let json = serde_json::to_string(&xpubs_with_paths).unwrap();
        CString::new(json).unwrap().into_raw()
    })
}

#[no_mangle]
pub unsafe extern "C" fn drop_wallet(wallet: *mut ElectrumSledWallet) {
    assert!(!wallet.is_null());
    let _: Box<ElectrumSledWallet> = Box::from_raw(wallet);
}

#[no_mangle]
///# Safety
/// deserialize consenus encoded base64 PSBT string
// TODO Turn this to a validate function ? is it ours, can we control any of it ? etc..
pub extern "C" fn consensus_b64_psbt_to_json_string(
    psbt: *const c_char,
) -> *mut BoxedResult<*mut c_char> {
    let psbt_str = required_str_from_cchar_ptr!(psbt);

    unwind_into_boxed_result!({
        let psbt =
            deserialize::<PartiallySignedTransaction>(&base64::decode(&psbt_str).unwrap()).unwrap();
        let json = serde_json::to_string(&psbt).unwrap();
        CString::new(json).unwrap().into_raw()
    })
}
