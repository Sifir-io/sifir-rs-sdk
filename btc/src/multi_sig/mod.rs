use crate::bdk::descriptor::IntoWalletDescriptor;
use crate::{sled, AddressIndex, Client, ElectrumBlockchain, Wallet};
use crate::{DerivedBip39Xprvs, XprvsWithPaths};
pub use bdk::bitcoin::util::bip32::{
    ChildNumber, DerivationPath, Error as Bip32Error, ExtendedPrivKey, ExtendedPubKey, Fingerprint,
    IntoDerivationPath,
};
pub use bdk::bitcoin::{secp256k1, Address, Network, OutPoint, PrivateKey, Script, Txid};
use bdk::keys::IntoDescriptorKey;
use bdk::keys::{DerivableKey, DescriptorKey};
use std::iter::Map;
use std::ops::RangeFrom;

pub struct MultiSigKey {
    pub key: String,
    pub position: u16,
}
impl MultiSigKey {
    pub fn new(key: String, position: u16) -> Self {
        MultiSigKey { key, position }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bdk::keys::ExtendedKey;
    use crate::bdk::miniscript::miniscript;
    use bdk::FeeRate;
    use std::rc::Rc;
    use std::str::FromStr;
    use std::sync::Arc;

    struct SifirWallet {} // TODO SifirWallet<T=WalletType>
    impl bdk::blockchain::Progress for SifirWallet {
        fn update(&self, progress: f32, message: Option<String>) -> Result<(), bdk::Error> {
            println!("progress is {} and message {:?}", progress, message);
            Ok(())
        }
    }

    #[test]
    fn gen_multi_sig() {
        // 1. Make some keys
        let derive_base = "m/44'/0'/0'";
        let network = Network::Testnet;
        let mut xpub_xprv_tuple = (1..)
            .map(|_| {
                let key = DerivedBip39Xprvs::new(
                    derive_base.into_derivation_path().unwrap(),
                    network,
                    1,
                    None,
                    None,
                )
                .unwrap();
                key
            })
            .map(|key| {
                let XprvsWithPaths(key, path, master_fp) = &key.xprv_w_paths[0];
                // Returns a tuple of closures that return cloned descriptors
                // TODO is all this cloning necessary to keep compiler happy? Clean this up
                let p1 = Arc::new(path.clone());
                let p2 = Arc::new(path.clone());
                let mf = master_fp.clone();
                let key1 = key.clone();
                let key2 = key.clone();
                let child_number = key.child_number.clone();
                (
                    move || {
                        let path = (*p1.clone()).clone();
                        let ex_xpub: ExtendedKey<miniscript::Segwitv0> =
                            key1.into_extended_key().unwrap();
                        let xpub =
                            ex_xpub.into_xpub(Network::Testnet, &secp256k1::Secp256k1::new());
                        xpub.clone()
                            .into_descriptor_key(
                                Some((mf, path.clone())),
                                "m/0".into_derivation_path().unwrap(),
                            )
                            .unwrap()
                    },
                    move || {
                        let path = (*p2.clone()).clone();
                        let ex_xprv: ExtendedKey<miniscript::Segwitv0> =
                            key2.into_extended_key().unwrap();
                        let xprv = ex_xprv.into_xprv(network).unwrap();
                        // Note: here use the path to apply to xprv, not full derivation path
                        (xprv.clone(), "m/0".into_derivation_path().unwrap())
                            .into_descriptor_key()
                            .unwrap()
                    },
                )
            });

        let (andriana_xpub, andriana_xprv) = xpub_xprv_tuple.next().unwrap();
        let (jose_xpub, jose_xprv) = xpub_xprv_tuple.next().unwrap();
        let (ahmed_xpub, ahmed_xprv) = xpub_xprv_tuple.next().unwrap();

        let andriana_wallet_desc = bdk::descriptor!(wsh(sortedmulti_vec(
            2,
            vec![andriana_xprv(), jose_xpub(), ahmed_xpub()]
        )))
        .unwrap();
        let jose_wallet_desc = bdk::descriptor!(wsh(sortedmulti_vec(
            2,
            vec![andriana_xpub(), jose_xprv(), ahmed_xpub()]
        )))
        .unwrap();
        let ahmed_wallet_desc = bdk::descriptor!(wsh(sortedmulti_vec(
            2,
            vec![andriana_xpub(), jose_xpub(), ahmed_xprv()]
        )))
        .unwrap();

        // Print out the wallet descriptors with secrets for testing
        //println!(
        //    "Ad:\r\n{:?}\r\n Jo:\r\n{:?}\r\nAh:\r\n{:?}\r\n",
        //    andriana_wallet_desc
        //        .0
        //        .to_string_with_secret(&andriana_wallet_desc.1),
        //    jose_wallet_desc
        //        .0
        //        .to_string_with_secret(&jose_wallet_desc.1),
        //    ahmed_wallet_desc
        //        .0
        //        .to_string_with_secret(&ahmed_wallet_desc.1),
        //);

        // 4. Wallet
        let db = sled::open("/tmp/bdk").unwrap();

        let synced_wallets: Vec<_> =
            vec![andriana_wallet_desc, jose_wallet_desc, ahmed_wallet_desc]
                .into_iter()
                .enumerate()
                .map(|(i, desc)| {
                    db.drop_tree(format!("multi_sig_test_{}", i)).unwrap();
                    let tree = db.open_tree(format!("multi_sig_test_{}", i)).unwrap();

                    let descriptor = desc
                        .into_wallet_descriptor(&secp256k1::Secp256k1::new(), Network::Testnet)
                        .unwrap();
                    Wallet::new(
                        descriptor,
                        None,
                        network,
                        tree,
                        ElectrumBlockchain::from(
                            Client::new("ssl://electrum.blockstream.info:60002").unwrap(),
                        ),
                    )
                    .unwrap()
                })
                .map(|wallet| {
                    wallet.sync(SifirWallet {}, Some(100)).unwrap();
                    wallet
                })
                .collect();

        let ariana_add = synced_wallets[0]
            .get_address(AddressIndex::LastUnused)
            .unwrap();
        let jose_add = synced_wallets[1]
            .get_address(AddressIndex::LastUnused)
            .unwrap();
        let ahmad_add = synced_wallets[2]
            .get_address(AddressIndex::LastUnused)
            .unwrap();
        //println!(
        //    "Add1: {} \r\n Add2: {} \r\n Add3: {} \r\n",
        //    ariana_add, jose_add, ahmad_add
        //);
        assert_eq!(ariana_add, jose_add);
        assert_eq!(jose_add, ahmad_add);
    }

    #[test]
    fn sign_psbt_txn() {
        // send back to  https://testnet-faucet.mempool.co/
        let rcvr_address = "mkHS9ne12qx9pS9VojpwU5xtRd4T7X7ZUt";

        let adoni_wallet=        Wallet::new(
                    "wsh(sortedmulti(2,tprv8hgp5kTScgDMnq4Hf8SBDpruHTiCS74JuYwhWugXvAh2oeaog68fcAR6F56nsG5nF16R2BDKoZJRCw8NzGFLdzJo9YNC9n1FBwpAWn9a8jW/0/*,[2499b0ca/44\'/0\'/0\'/0]tpubDFC7PgEVXDMBT2uFt4GPzPr3U15oF442smXrpSFCWjqH7MF7MxVgRsEd66DJyRsaKoNqSPH1jyN29ibtrdDGouuPDGonk19YMQqD8VSbzTV/0/*,[8b903d3e/44\'/0\'/0\'/0]tpubDEPJiJWFHiv7t9VcmdYVWW98WsbgqKZ7z2GrPoDPaexEK8nr8gy68WxJzMrqpg6GhjhpY7tASsbEzgjx6YD5yBX8XdB6W54e2FV5A1gbwYB/0/*))#nnhu2nf2",
                    None,
                    Network::Testnet,
                    bdk::database::MemoryDatabase::new(),
                    ElectrumBlockchain::from(
                        Client::new("ssl://electrum.blockstream.info:60002").unwrap(),
                    ),
                )
                .unwrap();
        let janis_wallet=        Wallet::new(
                    "wsh(sortedmulti(2,[50faf01e/44\'/0\'/0\'/0]tpubDENrEAVgm3u2gJ65Yn6mdEX1rVE8bSFDUrYUoRiqLSVRe8qaJUxFnf2xRAZd6gaLyrTwjCCKnvY6x5kZCFtYPgC3jmL7bwXb4v9bp6kdLvz/0/*,tprv8iW5FGCFNqfWZZsTzQboazBvtyZs5is8JTw5XvCu6U2tGrzLjZg6FNckuw816Ea7YtkGrRRPRxraJ552ng6dj4t5eHBSrhax83af4pqUDzu/0/*,[8b903d3e/44\'/0\'/0\'/0]tpubDEPJiJWFHiv7t9VcmdYVWW98WsbgqKZ7z2GrPoDPaexEK8nr8gy68WxJzMrqpg6GhjhpY7tASsbEzgjx6YD5yBX8XdB6W54e2FV5A1gbwYB/0/*))#87eltsvz",
                    None,
                    Network::Testnet,
                    bdk::database::MemoryDatabase::new(),
                    ElectrumBlockchain::from(
                        Client::new("ssl://electrum.blockstream.info:60002").unwrap(),
                    ),
                )
                .unwrap();
        let albert_wallet=        Wallet::new(
            "wsh(sortedmulti(2,[50faf01e/44\'/0\'/0\'/0]tpubDENrEAVgm3u2gJ65Yn6mdEX1rVE8bSFDUrYUoRiqLSVRe8qaJUxFnf2xRAZd6gaLyrTwjCCKnvY6x5kZCFtYPgC3jmL7bwXb4v9bp6kdLvz/0/*,[2499b0ca/44\'/0\'/0\'/0]tpubDFC7PgEVXDMBT2uFt4GPzPr3U15oF442smXrpSFCWjqH7MF7MxVgRsEd66DJyRsaKoNqSPH1jyN29ibtrdDGouuPDGonk19YMQqD8VSbzTV/0/*,tprv8hhGZtU19MESzgTpsysu76V1wr5kfzNDQig57HB6AP9qUeY5WJ9Vx2LSpBrs4C3eTdcB4wq4vCHo8DZaQkPjUWJByQFuybUy7KsAHZ8dJD9/0/*))#a4lh8slj",
            None,
            Network::Testnet,
            bdk::database::MemoryDatabase::new(),
            ElectrumBlockchain::from(
                Client::new("ssl://electrum.blockstream.info:60002").unwrap(),
            ),
        )
            .unwrap();

        adoni_wallet.sync(SifirWallet {}, Some(100)).unwrap();
        janis_wallet.sync(SifirWallet {}, Some(100)).unwrap();
        albert_wallet.sync(SifirWallet {}, Some(100)).unwrap();

        let adoni_address = adoni_wallet.get_address(AddressIndex::LastUnused).unwrap();
        let janis_address = janis_wallet.get_address(AddressIndex::LastUnused).unwrap();
        let albert_address = albert_wallet.get_address(AddressIndex::LastUnused).unwrap();

        assert_eq!(adoni_address, janis_address);
        assert_eq!(janis_address, albert_address);
        //
        // Send some sats to this scripts address
        //println!(
        //    "adnoi: {}, janis: {} , albert: {}",
        //    adoni_address, janis_address, albert_address
        //);

        adoni_wallet.sync(SifirWallet {}, Some(100)).unwrap();
        janis_wallet.sync(SifirWallet {}, Some(100)).unwrap();
        albert_wallet.sync(SifirWallet {}, Some(100)).unwrap();

        let adoni_balance = adoni_wallet.get_balance().unwrap();
        let janis_balance = janis_wallet.get_balance().unwrap();
        let albert_balance = albert_wallet.get_balance().unwrap();

        //println!(
        //    "adnoi: {}, janis: {} , albert: {}",
        //    adoni_balance, janis_balance, albert_balance
        //);
        assert!(adoni_balance > 0);
        assert_eq!(adoni_balance, janis_balance);
        assert_eq!(janis_balance, albert_balance);

        let mut txn = janis_wallet.build_tx();
        txn.add_recipient(
            Address::from_str(rcvr_address)
                .unwrap()
                .payload
                .script_pubkey(),
            1000,
        )
        .fee_rate(FeeRate::from_sat_per_vb(1.0))
        .enable_rbf();

        let (psbt, _tx_details) = txn.finish().unwrap();
        let (psbt_signed, finished) = janis_wallet.sign(psbt, None).unwrap();

        assert!(!finished);
        // Try to make PSBT to spend ?
        let (psbt_dually_signed, finished) = albert_wallet.sign(psbt_signed, None).unwrap();
        assert!(finished);
    }
}
