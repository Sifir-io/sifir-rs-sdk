use bdk::bitcoin::consensus::encode::{
    deserialize, serialize, serialize_hex, Error as BitcoinError,
};
use bdk::bitcoin::util::address::Error as BitcoinAddressError;
pub use bdk::bitcoin::util::bip32::{
    ChildNumber, DerivationPath, Error as Bip32Error, ExtendedPrivKey, ExtendedPubKey, Fingerprint,
    IntoDerivationPath,
};
pub use bdk::bitcoin::{secp256k1, Address, Network, OutPoint, PrivateKey, Script, Txid};
use bdk::blockchain::{log_progress, Blockchain, ElectrumBlockchain, Progress, ProgressData};
use bdk::database::{BatchDatabase, MemoryDatabase};
use bdk::descriptor::IntoWalletDescriptor;
use bdk::electrum_client::Client;
use bdk::keys::bip39::{Language, Mnemonic, MnemonicType};
use bdk::keys::{
    DerivableKey, DescriptorKey, ExtendedKey, GeneratableKey, GeneratedKey, KeyError, ScriptContext,
};
use bdk::miniscript::miniscript;
pub use bdk::{sled, FeeRate, TxBuilder, Wallet};
use rand::{thread_rng, RngCore};
use serde::{Deserialize, Serialize};
use serde_json;

use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize)]
pub struct WalletDescriptors {
    network: Network,
    external: String,
    internal: String,
    public: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct WalletCfg {
    name: String,
    descriptors: WalletDescriptors,
    address_look_ahead: u32,
    db_path: Option<String>,
}

#[repr(C)]
#[derive(Debug, Serialize, Deserialize)]
pub struct XprvsWithPaths(ExtendedPrivKey, DerivationPath, Fingerprint);

#[repr(C)]
#[derive(Debug, Serialize, Deserialize)]
pub struct DerivedBip39Xprvs {
    phrase: String,
    master_xprv: ExtendedPrivKey,
    xprv_w_paths: Vec<XprvsWithPaths>,
}

#[derive(Error, Debug)]
pub enum BtcErrors {
    #[error("General BDK:")]
    BdkError(#[from] bdk::Error),
    #[error("BDK Key Error:")]
    BdkKeyError(#[from] bdk::keys::KeyError),
    #[error("Bip32 error:")]
    Bip32Error(#[from] Bip32Error),
    #[error("Io Error:")]
    IoError(#[from] std::io::Error),
    #[error("Descriptor Error:")]
    DescriptorError(#[from] bdk::descriptor::error::Error),
    #[error("BitcoinError Error:")]
    BitcoinError(#[from] BitcoinError),
    #[error("Bitcoin Address Error Error:")]
    BitcoinAddressError(#[from] BitcoinAddressError),
    #[error("Expected value missing for {:?}",.0)]
    EmptyOption(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[derive(Debug, Serialize, Deserialize)]
enum FeeType {
    Abs,
    Rate,
}
#[derive(Debug, Serialize, Deserialize)]
enum SpendChangePolicy {
    Yes,
    No,
    OnlyChange,
}

/// Txn paramteres Easy to serialize into JSON and send across FFI
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTx {
    recipients: Vec<(String, u64)>,
    fee_type: FeeType,
    fee: f32,
    spend_change: SpendChangePolicy,
    enable_rbf: bool,
}

/// Converts a CreateTx struct into a BDK TxnBuilder and finalizes it
pub fn create_tx_to_wallet_txn<B: Blockchain, D: BatchDatabase>(
    wallet: &Wallet<B, D>,
    tx: CreateTx,
) -> Result<
    (
        bdk::bitcoin::util::psbt::PartiallySignedTransaction,
        bdk::TransactionDetails,
    ),
    BtcErrors,
> {
    let mut txn = wallet.build_tx();
    // strings to addresses
    let rcpts: Result<Vec<(Script, u64)>, BtcErrors> = tx
        .recipients
        .into_iter()
        .map(|(addr, amount)| {
            let address = Address::from_str(addr.as_str())?.script_pubkey();
            Ok((address, amount))
        })
        .collect();

    txn.set_recipients(rcpts?);
    match tx.fee_type {
        FeeType::Abs => txn.fee_absolute(tx.fee as u64),
        FeeType::Rate => txn.fee_rate(FeeRate::from_sat_per_vb(tx.fee)),
    };

    match tx.spend_change {
        SpendChangePolicy::No => {
            txn.do_not_spend_change();
        }
        SpendChangePolicy::OnlyChange => {
            txn.only_spend_change();
        }
        _ => (),
    };

    txn.enable_rbf();
    txn.finish().map_err(|err| BtcErrors::BdkError(err))
}
pub type ElectrumMemoryWallet = Wallet<ElectrumBlockchain, MemoryDatabase>;
pub type ElectrumSledWallet = Wallet<ElectrumBlockchain, sled::Tree>;

impl From<WalletCfg> for ElectrumMemoryWallet {
    fn from(cfg: WalletCfg) -> ElectrumMemoryWallet {
        let secp = &secp256k1::Secp256k1::new();
        Wallet::new(
            cfg.descriptors
                .external
                .as_str()
                .into_wallet_descriptor(&secp, cfg.descriptors.network)
                .unwrap(),
            Some(
                cfg.descriptors
                    .internal
                    .as_str()
                    .into_wallet_descriptor(&secp, cfg.descriptors.network)
                    .unwrap(),
            ),
            cfg.descriptors.network,
            MemoryDatabase::new(),
            ElectrumBlockchain::from(Client::new("ssl://electrum.blockstream.info:60002").unwrap()),
        )
        .unwrap()
    }
}

impl From<WalletCfg> for ElectrumSledWallet {
    fn from(cfg: WalletCfg) -> ElectrumSledWallet {
        let secp = &secp256k1::Secp256k1::new();
        let db = sled::open(cfg.db_path.expect("Missing db path")).unwrap();
        let tree = db.open_tree(cfg.name).unwrap();
        Wallet::new(
            cfg.descriptors
                .external
                .as_str()
                .into_wallet_descriptor(&secp, cfg.descriptors.network)
                .unwrap(),
            Some(
                cfg.descriptors
                    .internal
                    .as_str()
                    .into_wallet_descriptor(&secp, cfg.descriptors.network)
                    .unwrap(),
            ),
            cfg.descriptors.network,
            tree,
            ElectrumBlockchain::from(Client::new("ssl://electrum.blockstream.info:60002").unwrap()),
        )
        .unwrap()
    }
}

///
/// Generate a new Bip39 mnemonic seed
/// Derive num_child from provided derive_base
///
/// FIXME
/// refactor this to
/// let mnemonic = Mnemonic::from_phrase(...)?;
/// let base_path = DerivationPath::from_str("m/0'");
/// let descriptors = (0..1)
///   .map(|index| ChildNumber::Normal { index }) // transform the index into a `ChildNumber`
///   .map(|child| base_path.extend(&[child]))    // create the full path (base path + this child)
///   .map(|full_path| bdk::descriptor!(wpkh((menmonic.clone(), full_path)))); // create a wpkh descriptor with the mnemonic and the full path
///
/// let (external_desc, ext_keymap, valid_network) = descriptors.next().unwrap()?;
/// let (internal_desc, int_keymap, _) = descriptors.next().unwrap()?;
/// SEE NOTES
/// TODO - Maybe i just need to use the correct key fingerprint ? Ie the root ?
impl DerivedBip39Xprvs {
    pub fn new(
        derive_base: DerivationPath,
        network: Network,
        num_child: usize,
        password: Option<String>,
        seed_phrase: Option<String>,
    ) -> Result<Self, BtcErrors> {
        let secp = secp256k1::Secp256k1::new();
        let (master_key, mnemonic_gen): (ExtendedKey<miniscript::BareCtx>, Mnemonic) =
            match seed_phrase {
                Some(phrase) => {
                    let mnemonic = Mnemonic::from_phrase(&phrase, Language::English)?;
                    (mnemonic.clone().into_extended_key()?, mnemonic)
                }
                None => {
                    let mnemonic: GeneratedKey<_, miniscript::BareCtx> =
                        Mnemonic::generate((MnemonicType::Words24, Language::English)).unwrap();
                    let mnemonic = mnemonic.into_key();
                    ((mnemonic.clone(), password).into_extended_key()?, mnemonic)
                }
            };

        let xprv_master = master_key
            .into_xprv(network)
            .ok_or(BtcErrors::EmptyOption("xprv_master was empty".into()))?;
        // derive n childs int/ext <derive_base>/n' and cast from Vec<Result> - Result<Vec>
        let xprv_w_paths: Result<Vec<XprvsWithPaths>, BtcErrors> = derive_base
            .normal_children()
            .map(|child_path| -> Result<XprvsWithPaths, BtcErrors> {
                // Path is relative to key, so here derive from master
                let extended_priv = xprv_master.derive_priv(&secp, &child_path)?;
                Ok(XprvsWithPaths(
                    extended_priv,
                    child_path,
                    xprv_master.fingerprint(&secp),
                ))
            })
            .take(num_child)
            .collect();

        Ok(DerivedBip39Xprvs {
            master_xprv: xprv_master,
            phrase: mnemonic_gen.into_phrase(),
            xprv_w_paths: xprv_w_paths?,
        })
    }
}

impl From<(Vec<XprvsWithPaths>, Network)> for WalletDescriptors {
    fn from((keys, network): (Vec<XprvsWithPaths>, Network)) -> Self {
        let mut descriptors = keys
            .iter()
            .map(|XprvsWithPaths(key, path, master_fp)| {
                let descriptor_key = key
                    .into_descriptor_key(Some((*master_fp, path.clone())), path.clone())
                    .unwrap();
                // TODO define the type of descriptor
                bdk::descriptor!(wpkh((descriptor_key))).unwrap()
            })
            .take(2);

        let (external_desc, ext_keymap, _) = descriptors.next().unwrap();
        let (internal_desc, int_keymap, _) = descriptors.next().unwrap();

        WalletDescriptors {
            external: external_desc.to_string_with_secret(&ext_keymap),
            internal: internal_desc.to_string_with_secret(&int_keymap),
            network,
            public: external_desc.to_string(),
        }
    }
}

pub fn generate_extended_priv_key(network: Network) -> Result<ExtendedPrivKey, Bip32Error> {
    let mut entropy = [0u8; secp256k1::constants::SECRET_KEY_SIZE];
    thread_rng().fill_bytes(&mut entropy);
    ExtendedPrivKey::new_master(network, &entropy)
}
pub fn generate_wif(network: Network) -> String {
    let mut entropy = [0u8; secp256k1::constants::SECRET_KEY_SIZE];
    thread_rng().fill_bytes(&mut entropy);
    PrivateKey {
        compressed: true,
        network,
        key: secp256k1::SecretKey::from_slice(&entropy).expect("Error passing"),
    }
    .to_wif()
}

/// @deprecated Before BDK had descriptor macros
/// Kept for test purposes
pub fn generate_pkh_descriptors(
    network: Network,
    key: Option<ExtendedPrivKey>,
) -> Result<WalletDescriptors, BtcErrors> {
    let extended_priv_key = match key {
        Some(key) => key,
        None => generate_extended_priv_key(network).unwrap(),
    };
    //  m/0
    let wallet = extended_priv_key
        .ckd_priv(
            &secp256k1::Secp256k1::new(),
            ChildNumber::Hardened { index: 0 },
        )
        .unwrap();
    // m/0'/0'
    let wallet_chain_int = wallet
        .ckd_priv(
            &secp256k1::Secp256k1::new(),
            ChildNumber::Hardened { index: 1 },
        )
        .unwrap();
    // m/0'/1'
    let wallet_chain_ext = wallet
        .ckd_priv(
            &secp256k1::Secp256k1::new(),
            ChildNumber::Hardened { index: 0 },
        )
        .unwrap();

    let wallet_chain_ext_pubkey =
        ExtendedPubKey::from_private(&secp256k1::Secp256k1::new(), &wallet_chain_ext);

    let descriptor_int = format!(
        "pkh({}/{}/*)",
        wallet_chain_int.to_string(),
        wallet_chain_int
            .child_number
            .to_string()
            .trim_end_matches("'")
    );
    let descriptor_ext = format!(
        "pkh({}/{}/*)",
        wallet_chain_ext.to_string(),
        wallet_chain_ext
            .child_number
            .to_string()
            .trim_end_matches("'")
    );
    let descriptor_ext_xpub = format!(
        "pkh([{}/44'/{}/{}]{}/{}/*)",
        wallet_chain_ext_pubkey.parent_fingerprint,
        wallet.child_number,
        wallet_chain_ext_pubkey.child_number,
        wallet_chain_ext_pubkey.to_string(),
        wallet_chain_ext
            .child_number
            .to_string()
            .trim_end_matches("'")
    );
    Ok(WalletDescriptors {
        network,
        external: descriptor_ext,
        internal: descriptor_int,
        public: descriptor_ext_xpub,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_a_harcoded_wallet() {
        let external_descriptor = "wpkh(tprv8ZgxMBicQKsPdy6LMhUtFHAgpocR8GC6QmwMSFpZs7h6Eziw3SpThFfczTDh5rW2krkqffa11UpX3XkeTTB2FvzZKWXqPY54Y6Rq4AQ5R8L/84'/0'/0'/0/*)";
        let internal_descriptor = "wpkh(tprv8ZgxMBicQKsPdy6LMhUtFHAgpocR8GC6QmwMSFpZs7h6Eziw3SpThFfczTDh5rW2krkqffa11UpX3XkeTTB2FvzZKWXqPY54Y6Rq4AQ5R8L/84'/0'/0'/1/*)";
        let wallet = Wallet::new(
            external_descriptor,
            Some(internal_descriptor),
            Network::Testnet,
            MemoryDatabase::new(),
            ElectrumBlockchain::from(Client::new("ssl://electrum.blockstream.info:60002").unwrap()),
        )
        .unwrap();
        let address = wallet.get_new_address().unwrap().address_type().unwrap();
        assert_eq!(format!("{}", address), "p2wpkh")
    }
    #[test]
    fn make_descriptors_manually() {
        let desc = generate_pkh_descriptors(Network::Testnet, None).unwrap();
        assert_eq!(desc.external.starts_with("pkh"), true);
        assert_eq!(desc.internal.starts_with("pkh"), true);
        assert_eq!(desc.public.contains("tpub"), true);
    }

    #[test]
    fn get_wallet_from_cfg() {
        let desc = generate_pkh_descriptors(Network::Testnet, None).unwrap();
        let wallet: ElectrumMemoryWallet = WalletCfg {
            name: String::from("my test"),
            descriptors: desc,
            address_look_ahead: 2,
            db_path: None,
        }
        .into();
        let address = wallet.get_new_address().unwrap().address_type().unwrap();
        assert_eq!(format!("{}", address), "p2pkh")
    }

    #[test]
    fn make_bip39_pkh_deterministic_wallet() {
        let test_mnemonic =
            "aim bunker wash balance finish force paper analyst cabin spoon stable organ";
        // create a new randomly generated mnemonic phrase
        let mnemonic = Mnemonic::from_phrase(test_mnemonic, Language::English).unwrap();

        let desc = generate_pkh_descriptors(
            Network::Testnet,
            Some(ExtendedPrivKey::new_master(Network::Testnet, mnemonic.entropy()).unwrap()),
        )
        .unwrap();

        let wallet: ElectrumMemoryWallet = WalletCfg {
            name: String::from("my test"),
            descriptors: desc,
            address_look_ahead: 2,
            db_path: None,
        }
        .into();
        let address = wallet.get_new_address().unwrap();
        assert_eq!("mnqdgsNu8p2YCUAqQcbm5AVBMRXjMAnw5y", address.to_string());
        assert_eq!(format!("{}", address.address_type().unwrap()), "p2pkh");
        // get the HD wallet seed
    }

    #[test]
    fn derive_path_with_bip39() {
        let secp = secp256k1::Secp256k1::new();
        let test_mnemonic =
            "aim bunker wash balance finish force paper analyst cabin spoon stable organ";
        let num_child = 2;
        // segwit/coin/account
        let derive_base = "m/44'/0'/0'";
        let network = Network::Bitcoin;

        let mnemonic = Mnemonic::from_phrase(test_mnemonic, Language::English).unwrap();
        let key: ExtendedKey<miniscript::BareCtx> = mnemonic.into_extended_key().unwrap();

        // master m
        let xprv_master = key.into_xprv(network).unwrap();
        assert_eq!(xprv_master.depth, 0);

        // wallet root m/0'
        let derive_path = String::from(derive_base).into_derivation_path().unwrap();
        assert_eq!(derive_path.to_string(), derive_base);

        let wallet_root_key = xprv_master.derive_priv(&secp, &derive_path).unwrap();
        assert_eq!(wallet_root_key.depth, 3);

        // from https://iancoleman.io/bip39/
        let expected_xprvs = ["xprvA1Rm2Dm6Zgjc6yLcH1vyS1VuykpPMKCQmymmYFv9kSvpfZ51y8G6wzaZVC6BtphuiDKEXcsENy3RbwLa3Nqwb9VBQvQagEG6J5EK76aTjmh","xprvA1Rm2Dm6Zgjc8zwffxi6Bb9dX5V14mvLRPVo72J3Q8C5BHRyACD7Ywk2L7ovf5fo8WcBQ7Janoba9fQXjXuY5wQaRfzj5ahZkPBZY449suQ"];

        // derive n childs int/ext m/0'/n'
        derive_path
            .normal_children()
            .map(|child_path| {
                (
                    // Note path is relative to key, so here derive from master
                    xprv_master.derive_priv(&secp, &child_path).unwrap(),
                    child_path,
                )
            })
            .take(num_child)
            .collect::<Vec<(ExtendedPrivKey, DerivationPath)>>()
            .into_iter()
            .enumerate()
            .for_each(|(i, (key, path))| {
                assert_eq!(format!("{}", path), format!("m/44'/0'/0'/{}", i));
                assert_eq!(key.to_string(), expected_xprvs[i]);
                assert_eq!(key.depth, 4);
                assert_eq!(key.parent_fingerprint, wallet_root_key.fingerprint(&secp));
            });
    }

    #[test]
    fn generate_a_bip39_wallet_with_n_keys_from_path() {
        let num_child = 2;
        // segwit/coin/account
        let derive_base = "m/44'/0'/0'";
        let network = Network::Testnet;
        let wallet_xprvs = DerivedBip39Xprvs::new(
            derive_base.into_derivation_path().unwrap(),
            network,
            2,
            Some(String::from("mypass")),
            None,
        )
        .unwrap();

        let descriptors: WalletDescriptors = (wallet_xprvs.xprv_w_paths, network).into();
        println!("Descr {:#?}", serde_json::to_string(&descriptors));
        let wallet_cfg = WalletCfg {
            name: String::from("my test"),
            descriptors,
            address_look_ahead: 2,
            db_path: None,
        };
        let wallet: ElectrumMemoryWallet = wallet_cfg.into();
        let address = wallet.get_new_address().unwrap();
        assert_eq!(format!("{}", address.address_type().unwrap()), "p2wpkh");
    }
    #[test]
    fn electrum_sled() {
        let num_child = 2;
        // segwit/coin/account
        let derive_base = "m/44'/0'/0'";
        let network = Network::Testnet;
        let wallet_xprvs = DerivedBip39Xprvs::new(
            derive_base.into_derivation_path().unwrap(),
            network,
            2,
            Some(String::from("mypass")),
            None,
        )
        .unwrap();

        let descriptors: WalletDescriptors = (wallet_xprvs.xprv_w_paths, network).into();
        let wallet_cfg = WalletCfg {
            name: String::from("mytest2"),
            descriptors,
            address_look_ahead: 2,
            db_path: Some(String::from("/tmp/sifir-bdk")),
        };
        let wallet: ElectrumSledWallet = wallet_cfg.into();
        let address = wallet.get_new_address().unwrap();
        assert_eq!(format!("{}", address.address_type().unwrap()), "p2wpkh");
    }

    #[test]
    fn txn_from_create_txn_json() {
        let rcvr_wallet_cfg:WalletCfg = serde_json::from_str( "{\"name\":\"my test\",\"descriptors\":{\"network\":\"testnet\",\"external\":\"wpkh(tprv8e5FKc3Mdn1ByJgZ2GBBA4DFZ2tAzmtquHHPhRtRFmq1M8a3je1DXhRu2Dnx6db3GKmavKbku5sdkcAzWBHwi1KVoNMi4V3oox4vfrvuyNs/0\'/0/*)\",\"internal\":\"wpkh([547f0cd3/0\'/1]tprv8e5FKc3Mdn1C3eHRQ4pBZR13wHTHpX1umHgPpQv9HDjA5MRUKmRQqjWic6gfSAp6CDyM8B3ur3jkayG7E8yG5eNj3ZcCEJnuaKa14Q9Tf9W/0\'/1/*)#63l9gmuk\",\"public\":\"wpkh([547f0cd3/0\'/0/0\']tpubDCYJ5ZRDkRcFtTQZzetaWVS6q52rs3RTAKXYMWEvGCR6Nb1LTFpdwGohYQ4f98aVE6NxYN3tru8kziP9vZhDYZYDd5VDERyFr8U5WeCbGHy/0/*)#knhduycc\"},\"address_look_ahead\":2}").unwrap();
        let sender_wallet_cfg:WalletCfg = serde_json::from_str( "{\"name\":\"my test_2\",\"descriptors\":{\"network\":\"testnet\",\"external\":\"wpkh(tprv8dWQe989ftsPNn9NddyKVri3GHs4voe2E4xS75oX9neHeQGCnbn2ru3o1mbxmW2SnNRtpMdaopc6GWftoGMyhKPX3zjCBTKU1Ckw6E6NmQL/0\'/0/*)\",\"internal\":\"wpkh([0776ff86/0\'/1]tprv8dWQe989ftsPRPriFR6w1cG3R2s5FBxXsDscygXe8RiaUQrRkA7J8FFJwTPRBoLia7fqVB8s87SQ5rLnVjbZfDpuorRkBBqrSHKVbhUMYmq/0\'/1/*)#ynv3pma4\",\"public\":\"wpkh([0776ff86/0\'/0/0\']tpubDCmXi7Tx4hixdHtw4WVgnSAsDJ4Q8oR3NSq6DYRmCC46hihokPaHo3RAdaSQza8sWtAU63zt5VgkgYt6tUmZQWqVZio5vptzgrNMmrRwcBF/0/*)#cn8qlh6k\"},\"address_look_ahead\":2}").unwrap();

        let rcvr_wallet: ElectrumMemoryWallet = rcvr_wallet_cfg.into();
        let sender_wallet: ElectrumMemoryWallet = sender_wallet_cfg.into();

        let recipients = (1..3)
            .map(|i| (rcvr_wallet.get_new_address().unwrap().to_string(), 1000 * i))
            .collect();
        let txn = CreateTx {
            recipients,
            fee_type: FeeType::Rate,
            fee: 4.0,
            spend_change: SpendChangePolicy::Yes,
            enable_rbf: true,
        };

        let txn_json = serde_json::to_string(&txn).unwrap();
        println!("txn json {}", txn_json);
        let txn: CreateTx = serde_json::from_str(&txn_json).unwrap();

        let wallet_txn = create_tx_to_wallet_txn(&sender_wallet, txn).unwrap();
        println!("txn wallet {:?} {:?}", wallet_txn.0, wallet_txn.1);
    }
    #[test]
    #[ignore]
    fn can_sync_wallet_and_sign_utxos() {
        let rcvr_wallet_cfg:WalletCfg = serde_json::from_str( "{\"name\":\"my test\",\"descriptors\":{\"network\":\"testnet\",\"external\":\"wpkh(tprv8e5FKc3Mdn1ByJgZ2GBBA4DFZ2tAzmtquHHPhRtRFmq1M8a3je1DXhRu2Dnx6db3GKmavKbku5sdkcAzWBHwi1KVoNMi4V3oox4vfrvuyNs/0\'/0/*)\",\"internal\":\"wpkh([547f0cd3/0\'/1]tprv8e5FKc3Mdn1C3eHRQ4pBZR13wHTHpX1umHgPpQv9HDjA5MRUKmRQqjWic6gfSAp6CDyM8B3ur3jkayG7E8yG5eNj3ZcCEJnuaKa14Q9Tf9W/0\'/1/*)#63l9gmuk\",\"public\":\"wpkh([547f0cd3/0\'/0/0\']tpubDCYJ5ZRDkRcFtTQZzetaWVS6q52rs3RTAKXYMWEvGCR6Nb1LTFpdwGohYQ4f98aVE6NxYN3tru8kziP9vZhDYZYDd5VDERyFr8U5WeCbGHy/0/*)#knhduycc\"},\"address_look_ahead\":2}").unwrap();
        let sender_wallet_cfg:WalletCfg = serde_json::from_str( "{\"name\":\"my test_2\",\"descriptors\":{\"network\":\"testnet\",\"external\":\"wpkh(tprv8dWQe989ftsPNn9NddyKVri3GHs4voe2E4xS75oX9neHeQGCnbn2ru3o1mbxmW2SnNRtpMdaopc6GWftoGMyhKPX3zjCBTKU1Ckw6E6NmQL/0\'/0/*)\",\"internal\":\"wpkh([0776ff86/0\'/1]tprv8dWQe989ftsPRPriFR6w1cG3R2s5FBxXsDscygXe8RiaUQrRkA7J8FFJwTPRBoLia7fqVB8s87SQ5rLnVjbZfDpuorRkBBqrSHKVbhUMYmq/0\'/1/*)#ynv3pma4\",\"public\":\"wpkh([0776ff86/0\'/0/0\']tpubDCmXi7Tx4hixdHtw4WVgnSAsDJ4Q8oR3NSq6DYRmCC46hihokPaHo3RAdaSQza8sWtAU63zt5VgkgYt6tUmZQWqVZio5vptzgrNMmrRwcBF/0/*)#cn8qlh6k\"},\"address_look_ahead\":2}").unwrap();

        let rcvr_wallet: ElectrumMemoryWallet = rcvr_wallet_cfg.into();
        let sender_wallet: ElectrumMemoryWallet = sender_wallet_cfg.into();

        println!("rcvr add {}", rcvr_wallet.get_new_address().unwrap());
        struct SifirWallet {
            version: String,
        }
        // TODO SifirWallet<T=WalletType>
        impl SifirWallet {
            pub fn new() -> Self {
                SifirWallet {
                    version: "0.0.1".into(),
                }
            }
        }
        impl Progress for SifirWallet {
            fn update(&self, progress: f32, message: Option<String>) -> Result<(), bdk::Error> {
                println!("progress is {} and message {:?}", progress, message);
                Ok(())
            }
        };
        let sync_result = sender_wallet.sync(SifirWallet::new(), Some(100));
        sync_result.unwrap();

        let balance = sender_wallet.get_balance().unwrap();
        assert!(balance > 100);
        let mut txn = sender_wallet.build_tx();
        txn.add_recipient(rcvr_wallet.get_new_address().unwrap().script_pubkey(), 1000)
            .fee_rate(FeeRate::from_sat_per_vb(5.0))
            .do_not_spend_change()
            .enable_rbf();

        let (psbt, tx_details) = txn.finish().unwrap();
        let (psbt_signed, finished) = sender_wallet.sign(psbt, None).unwrap();
        assert!(finished);
        let txn_id = sender_wallet.broadcast(psbt_signed.extract_tx()).unwrap();
    }
}
