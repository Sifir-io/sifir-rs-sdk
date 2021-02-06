use bdk::bitcoin::util::bip32::{
    ChildNumber, Error as Bip32Error, ExtendedPrivKey, ExtendedPubKey,
};
pub use bdk::bitcoin::{secp256k1, Network, PrivateKey};
use bdk::blockchain::ElectrumBlockchain;
use bdk::database::MemoryDatabase;
use bdk::descriptor;
use bdk::descriptor::ToWalletDescriptor;
use bdk::electrum_client::Client;
use bdk::keys::{GeneratableKey, GeneratedKey};
use bdk::Wallet;
use bip39::{Language, Mnemonic, MnemonicType, Seed};
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
        Wallet::new(
            cfg.descriptors
                .external
                .as_str()
                .to_wallet_descriptor(cfg.descriptors.network)
                .unwrap(),
            Some(
                cfg.descriptors
                    .internal
                    .as_str()
                    .to_wallet_descriptor(cfg.descriptors.network)
                    .unwrap(),
            ),
            Network::Testnet,
            MemoryDatabase::new(),
            ElectrumBlockchain::from(Client::new("ssl://electrum.blockstream.info:60002").unwrap()),
        )
        .unwrap()
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

pub fn generate_wallet_descriptors(
    network: Network,
) -> Result<WalletDescriptors, descriptor::error::Error> {
    let extended_priv_key = generate_extended_priv_key(network)?;
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
    fn make_descriptors() {
        let desc = generate_wallet_descriptors(Network::Testnet).unwrap();
        assert_eq!(desc.external.starts_with("pkh"), true);
        assert_eq!(desc.internal.starts_with("pkh"), true);
        assert_eq!(desc.public.contains("tpub"), true);
    }

    #[test]
    fn get_wallet_from_cfg() {
        let desc = generate_wallet_descriptors(Network::Testnet).unwrap();
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
    fn test_keys_bip39_mnemonic() {
        let mnemonic =
            "aim bunker wash balance finish force paper analyst cabin spoon stable organ";
        let mnemonic_type = MnemonicType::Type12Words;
        /// create a new randomly generated mnemonic phrase
        let mnemonic = match Mnemonic::new(mnemonic_type, Language::English, "") {
            Ok(b) => b,
            Err(e) => {
                println!("e: {}", e);
                return;
            }
        };

        /// get the phrase as a string
        let phrase = mnemonic.get_string();
        println!("phrase: {}", phrase);
        /// get the HD wallet seed
        let seed = mnemonic.get_seed();
        // get the HD wallet seed as raw bytes
        let seed_bytes: &[u8] = seed.as_ref();
        // get the HD wallet seed as a hex string
        let seed_hex: &str = seed.as_hex();
        // get an owned Seed instance
        let owned_seed: Seed = seed.to_owned();
        let mnemonic = Mnemonic::from_phrase(mnemonic, Language::English).unwrap();
        let path = bdk::DerivationPath::from_str("m/44'/0'/0'/0").unwrap();
        let key = (mnemonic, path);
        let (desc, keys, networks) = bdk::descriptor!(wpkh(key)).unwrap();
        assert_eq!(desc.to_string(), "wpkh([be83839f/44'/0'/0']xpub6DCQ1YcqvZtSwGWMrwHELPehjWV3f2MGZ69yBADTxFEUAoLwb5Mp5GniQK6tTp3AgbngVz9zEFbBJUPVnkG7LFYt8QMTfbrNqs6FNEwAPKA/0/*)");
        assert_eq!(keys.len(), 1);
        assert_eq!(networks.len(), 3);
    }
}
