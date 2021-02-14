use bdk::bitcoin::util::bip32::{
    ChildNumber, DerivationPath, Error as Bip32Error, ExtendedPrivKey, Fingerprint,
    IntoDerivationPath,
};
pub use bdk::bitcoin::{secp256k1, PrivateKey};
use bdk::blockchain::ElectrumBlockchain;
use bdk::database::MemoryDatabase;
use bdk::descriptor;
use bdk::descriptor::ToWalletDescriptor;
use bdk::electrum_client::Client;
use bdk::keys::{DescriptorKey, ToDescriptorKey};
use bdk::keys::{KeyError, ScriptContext};

use anyhow::Result;
use bdk::bitcoin::consensus::encode::{deserialize, serialize, serialize_hex};
use bdk::bitcoin::hashes::hex::FromHex;
use bdk::bitcoin::secp256k1::Secp256k1;
use bdk::bitcoin::util::bip32::ExtendedPubKey;
use bdk::bitcoin::util::psbt::PartiallySignedTransaction;
use bdk::bitcoin::{Address, Network, OutPoint, Script, Txid};
use bdk::blockchain::{log_progress, Blockchain};
use bdk::database::BatchDatabase;
use bdk::keys::bip39::{Language, Mnemonic, MnemonicType};
use bdk::keys::{DerivableKey, ExtendedKey, GeneratableKey, GeneratedKey};
use bdk::miniscript::miniscript;
use bdk::Error;
use bdk::{FeeRate, KeychainKind, Wallet};

use rand::{thread_rng, RngCore};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct WalletDescriptors {
    network: Network,
    external: String,
    internal: String,
    public: String,
}
#[derive(Serialize, Deserialize)]
pub struct WalletCfg {
    name: String,
    descriptors: WalletDescriptors,
    address_look_ahead: u32,
}

type ElectrumMemoryWallet = Wallet<ElectrumBlockchain, MemoryDatabase>;

impl From<WalletCfg> for ElectrumMemoryWallet {
    fn from(cfg: WalletCfg) -> ElectrumMemoryWallet {
        let secp = &secp256k1::Secp256k1::new();
        Wallet::new(
            cfg.descriptors
                .external
                .as_str()
                .to_wallet_descriptor(&secp, cfg.descriptors.network)
                .unwrap(),
            Some(
                cfg.descriptors
                    .internal
                    .as_str()
                    .to_wallet_descriptor(&secp, cfg.descriptors.network)
                    .unwrap(),
            ),
            cfg.descriptors.network,
            MemoryDatabase::new(),
            ElectrumBlockchain::from(Client::new("ssl://electrum.blockstream.info:60002").unwrap()),
        )
        .unwrap()
    }
}
struct DerivedBip39Xprvs {
    phrase: String,
    xprv_w_paths: Vec<(ExtendedPrivKey, DerivationPath)>,
}
///
/// Generate a new Bip39 mnemonic seed
/// Derive num_child from provided derive_base
///
impl DerivedBip39Xprvs {
    fn new(
        derive_base: DerivationPath,
        network: Network,
        num_child: usize,
        password: Option<String>,
    ) -> Result<Self> {
        let secp = secp256k1::Secp256k1::new();
        let mnemonic: GeneratedKey<_, miniscript::BareCtx> =
            Mnemonic::generate((MnemonicType::Words24, Language::English)).unwrap();
        let mnemonic = mnemonic.into_key();
        let master_key: ExtendedKey<miniscript::BareCtx> =
            (mnemonic.clone(), password).into_extended_key()?;
        // FIXME propgration
        let xprv_master = master_key.into_xprv(network).unwrap();
        // derive n childs int/ext m/0'/n'
        let xprv_w_paths = derive_base
            .normal_children()
            .map(|child_path| {
                (
                    // Path is relative to key, so here derive from master
                    xprv_master.derive_priv(&secp, &child_path).unwrap(),
                    child_path,
                )
            })
            .take(num_child)
            .collect::<Vec<(ExtendedPrivKey, DerivationPath)>>();

        Ok(DerivedBip39Xprvs {
            phrase: mnemonic.into_phrase(),
            xprv_w_paths,
        })
    }
}

impl From<Vec<(ExtendedPrivKey, DerivationPath)>> for WalletDescriptors {
    fn from(keys: Vec<(ExtendedPrivKey, DerivationPath)>) -> Self {
        let mut descriptors = keys
            .iter()
            .map(|(key, path)| {
                let descriptor_key = key
                    .into_descriptor_key(Some((key.parent_fingerprint, path.clone())), path.clone())
                    .unwrap();
                // FIXME define the type of descriptor
                bdk::descriptor!(wpkh((descriptor_key))).unwrap()
            })
            .take(2);

        let (external_desc, ext_keymap, valid_network) = descriptors.next().unwrap();
        let (internal_desc, int_keymap, _) = descriptors.next().unwrap();

        WalletDescriptors {
            external: external_desc.to_string_with_secret(&ext_keymap),
            internal: internal_desc.to_string_with_secret(&int_keymap),
            // FIXME
            network: Network::Bitcoin,
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

pub fn generate_pkh_descriptors(
    network: Network,
    key: Option<ExtendedPrivKey>,
) -> Result<WalletDescriptors, descriptor::error::Error> {
    let extended_priv_key = match (key) {
        Some(key) => key,
        None => generate_extended_priv_key(network)?,
    };
    //  m/0
    let wallet = extended_priv_key.ckd_priv(
        &secp256k1::Secp256k1::new(),
        ChildNumber::Hardened { index: 0 },
    )?;
    // m/0'/0'
    let wallet_chain_int = wallet.ckd_priv(
        &secp256k1::Secp256k1::new(),
        ChildNumber::Hardened { index: 1 },
    )?;
    // m/0'/1'
    let wallet_chain_ext = wallet.ckd_priv(
        &secp256k1::Secp256k1::new(),
        ChildNumber::Hardened { index: 0 },
    )?;

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
        let derive_base = "m/0'";
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
        assert_eq!(wallet_root_key.depth, 1);

        // from https://iancoleman.io/bip39/
        let expected_xprvs = ["xprv9wvPSwhTzAefWQkC6da4xVZm2mJ267e31tbgsa7hbzY31wS9fQJGbDzNuN3dBAL1fPDvwwZJj1A2a48Gt3DBKaa463axRgPURN5Jgykf78W","xprv9wvPSwhTzAefXwZHxdEMEmgQoghA6GZR1ur1EigGRJtzRGu4C5Lz7qX6tEvg9ajNgthNyeixs3mKKVc9rNgTYJzqEiQ28rNkctpX5QTncw5"];

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
                assert_eq!(format!("{}", path), format!("m/0'/{}", i));
                assert_eq!(key.to_string(), expected_xprvs[i]);
                assert_eq!(key.depth, 2);
                assert_eq!(key.parent_fingerprint, wallet_root_key.fingerprint(&secp));
            });
    }

    #[test]
    fn generate_a_bip39_wallet_with_n_keys_from_path() {
        let num_child = 2;
        // segwit/coin/account
        let derive_base = "m/0'";
        let network = Network::Bitcoin;
        let wallet_xprvs = DerivedBip39Xprvs::new(
            "m/0'".into_derivation_path().unwrap(),
            Network::Bitcoin,
            2,
            Some(String::from("mypass")),
        )
        .unwrap();

        let wallet: ElectrumMemoryWallet = WalletCfg {
            name: String::from("my test"),
            descriptors: wallet_xprvs.xprv_w_paths.into(),
            address_look_ahead: 2,
        }
        .into();

        let address = wallet.get_new_address().unwrap();
        assert_eq!(format!("{}", address.address_type().unwrap()), "p2wpkh");
    }
}
