use crate::bdk::descriptor::IntoWalletDescriptor;
use crate::AnyDescriptorCfg;
use crate::{sled, AddressIndex, Client, ElectrumBlockchain, Wallet};
use crate::{
    DerivedBip39Xprvs, ElectrumMemoryWallet, ElectrumSledWallet, WalletDescriptors, XprvsWithPaths,
    XpubsWithPaths,
};
pub use bdk::bitcoin::util::bip32::{
    ChildNumber, DerivationPath, Error as Bip32Error, ExtendedPrivKey, ExtendedPubKey, Fingerprint,
    IntoDerivationPath,
};
pub use bdk::bitcoin::{secp256k1, Address, Network, OutPoint, PrivateKey, Script, Txid};
use bdk::keys::{DerivableKey, DescriptorKey, IntoDescriptorKey};
use serde::{Deserialize, Serialize};
use std::iter::Map;
use std::ops::RangeFrom;
use std::str::FromStr;

#[derive(Debug, Serialize, Deserialize)]
pub enum MultiSigKey {
    Xpub(XpubsWithPaths),
    Xprv(XprvsWithPaths),
}

#[derive(Debug, Serialize, Deserialize)]
///
///  Serializes as:
///{
///  "descriptors": [
///    {
///      "Xprv": [
///        "tprv8hb2jMkXPyzimyaTaQ9tTH2xj7CcQENS3uMdNptCyGmDgSbFA2q7Zfpyjs3kf96Ecmascxp2bRg1ztSXGGY3jhzT1N5chXgHUcRwWAAh7kY",
///        "m/0",
///        "ff31a959"
///      ]
///    },
///    {
///      "Xpub": [
///        "tpubDFSuJXy4vxC6vX3o1yNZjmdR7T7qS2FgbtqhHSvjNMjyXLHNJk9XzTqCPbVrbevbYyasY6wnS96s5Er4xkNosm3pcuyFH9LUxPUavJ2EZSC",
///        "m/44'/0'/0'/0",
///        "77306a4c"
///      ]
///    },
///    {
///      "Xpub": [
///        "tpubDEYM383BbDXgPSpGmBWcdDCDo5HbREUBPVeUuyypBXpyQsMGykfGA2AURtuHbaN7ktrcbyct665m6VbtyQKsQD17Vp7yavVwdyGQ87659RR",
///        "m/44'/0'/0'/0",
///        "d22d870c"
///      ]
///    }
///  ],
///  "network": "testnet",
///  "quorom": 2
///}
pub struct MultiSigCfg {
    descriptors: Vec<MultiSigKey>,
    network: Network,
    quorom: i32,
}

impl From<MultiSigCfg> for WalletDescriptors {
    fn from(
        MultiSigCfg {
            descriptors,
            quorom,
            network,
        }: MultiSigCfg,
    ) -> Self {
        let keys = descriptors
            .into_iter()
            .map(|key| {
                match key {
                    MultiSigKey::Xpub(xpub_w_paths) => xpub_w_paths
                        .0
                        .into_descriptor_key(
                            Some((xpub_w_paths.2, xpub_w_paths.1.clone())),
                            "m/0".into_derivation_path().unwrap(),
                        )
                        .unwrap(),
                    MultiSigKey::Xprv(xprv_w_paths) =>
                    // Note: here use the path to apply to xprv, not full derivation path
                    {
                        (xprv_w_paths.0, xprv_w_paths.1)
                            .into_descriptor_key()
                            .unwrap()
                    }
                }
            })
            .collect();

        let (multi_sig_desc, multi_key_map, _) =
            // TODO accept input of descriptor type
            bdk::descriptor!(wsh(sortedmulti_vec(quorom as usize, keys))).unwrap();

        WalletDescriptors {
            external: multi_sig_desc.to_string_with_secret(&multi_key_map),
            internal: None,
            network,
            public: multi_sig_desc.to_string(),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::bdk::keys::ExtendedKey;
    use crate::bdk::miniscript::miniscript;
    use crate::WalletCfg;
    use bdk::{FeeRate, SignOptions};
    use std::str::FromStr;

    struct SifirWallet {} // TODO SifirWallet<T=WalletType>
    impl bdk::blockchain::Progress for SifirWallet {
        fn update(&self, progress: f32, message: Option<String>) -> Result<(), bdk::Error> {
            println!("progress is {} and message {:?}", progress, message);
            Ok(())
        }
    }

    #[test]
    fn gen_multisig_wallets() {
        // 1. Make some keys
        let derive_base = "m/44'/0'/0'";
        let network = Network::Testnet;
        // Returns a tuple of closures that generate an Xpub and Xprv for each key
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
                // TODO is all this cloning necessary to keep compiler happy? Clean this up
                let p1 = path.clone();
                let mf = master_fp.clone();
                let mf2 = master_fp.clone();
                let key1 = key.clone();
                let key2 = key.clone();
                (
                    move || {
                        let path = p1.clone();
                        let ex_xpub: ExtendedKey<miniscript::Segwitv0> =
                            key1.into_extended_key().unwrap();
                        let xpub =
                            ex_xpub.into_xpub(Network::Testnet, &secp256k1::Secp256k1::new());
                        XpubsWithPaths(xpub, path, mf)
                    },
                    move ||
                        // Note to self: Path is relative to xprv (ie from here), which is m/0 here (NOT the full path used to get to this key)
                        XprvsWithPaths(key2, DerivationPath::from_str("m/0").unwrap(), mf2),
                )
            });

        let (andriana_xpub, andriana_xprv) = xpub_xprv_tuple.next().unwrap();
        let (jose_xpub, jose_xprv) = xpub_xprv_tuple.next().unwrap();
        let (ahmed_xpub, ahmed_xprv) = xpub_xprv_tuple.next().unwrap();
        // 2. Construct some wallet cfg
        let adriana_wallet_cfg = WalletCfg {
            name: String::from("adriana_wallet"),
            db_path: None,
            descriptors: AnyDescriptorCfg::WshMultiSorted(MultiSigCfg {
                descriptors: vec![
                    MultiSigKey::Xprv(andriana_xprv()),
                    MultiSigKey::Xpub(jose_xpub()),
                    MultiSigKey::Xpub(ahmed_xpub()),
                ],
                network: Network::Testnet,
                quorom: 2,
            })
            .into(),
            address_look_ahead: 1,
            server_uri: None,
        };

        let jose_wallet_cfg = WalletCfg {
            name: String::from("jose_wallet"),
            db_path: None,
            descriptors: AnyDescriptorCfg::WshMultiSorted(MultiSigCfg {
                descriptors: vec![
                    MultiSigKey::Xpub(andriana_xpub()),
                    MultiSigKey::Xprv(jose_xprv()),
                    MultiSigKey::Xpub(ahmed_xpub()),
                ],
                network: Network::Testnet,
                quorom: 2,
            })
            .into(),
            address_look_ahead: 1,
            server_uri: None,
        };
        let ahmed_wallet_cfg = WalletCfg {
            name: String::from("ahmed_wallet"),
            db_path: None,
            descriptors: AnyDescriptorCfg::WshMultiSorted(MultiSigCfg {
                descriptors: vec![
                    MultiSigKey::Xpub(andriana_xpub()),
                    MultiSigKey::Xpub(jose_xpub()),
                    MultiSigKey::Xprv(ahmed_xprv()),
                ],

                network: Network::Testnet,
                quorom: 2,
            })
            .into(),
            address_look_ahead: 1,
            server_uri: None,
        };
        println!(
            "{} \r\n {} \r\n {} \r\n",
            serde_json::to_string(&adriana_wallet_cfg).unwrap(),
            serde_json::to_string(&jose_wallet_cfg).unwrap(),
            serde_json::to_string(&ahmed_wallet_cfg).unwrap()
        );

        // 3. Wallet instances and sync
        let synced_wallets: Vec<_> = vec![adriana_wallet_cfg, jose_wallet_cfg, ahmed_wallet_cfg]
            .into_iter()
            .map(ElectrumMemoryWallet::from)
            .map(|wallet| {
                wallet.sync(SifirWallet {}, Some(100)).unwrap();
                wallet
            })
            .collect();

        // 4. Make sure we generate the same addresses

        let ariana_add = synced_wallets[0]
            .get_address(AddressIndex::LastUnused)
            .unwrap();
        let jose_add = synced_wallets[1]
            .get_address(AddressIndex::LastUnused)
            .unwrap();
        let ahmad_add = synced_wallets[2]
            .get_address(AddressIndex::LastUnused)
            .unwrap();
        assert_eq!(ariana_add, jose_add);
        assert_eq!(jose_add, ahmad_add);
        // println!("{}", ariana_add);
    }

    #[test]
    fn deserialize_multsig_and_sign() {
        // send back to  https://testnet-faucet.mempool.co/
        // let rcvr_address = "mkHS9ne12qx9pS9VojpwU5xtRd4T7X7ZUt";
        let rcvr_address = "tb1qcpvp8fwv23egkee7ld2dt9ndymcyhl58g4fvq479tsr62u5mjakq2r8gke";

        // Deserialize savde wallets
        let adriana_wallet: ElectrumMemoryWallet = serde_json::from_str::<WalletCfg>("{\"name\":\"adriana_wallet\",\"descriptors\":{\"network\":\"testnet\",\"external\":\"wsh(sortedmulti(2,tprv8ip37K5SBQqZcmpaH72wSNfAAXPt5UPUVnWHPTkPqgvQCm5kFRkQL9k8fmXV7xG9nB3eJxi2SKVGtRQspG2NRfkyj5dseVPaznx4saX8KQp/0/*,[011aed20/44'/0'/0'/0]tpubDFZR9wnXt7a1fUVj2jEJw7E37pju124qjEfDDG9efozibH7T5BYtjwbjHtxHDK9q2nqsTYn25CngM2XUpXd1oZWCByWjkCG1YY1wnRvPTb8/0/*,[83488a89/44'/0'/0'/0]tpubDFi8BjbFcmJ3owMcf6SP2ZwZfYFWqi1C9PUv3EdorHCeipRycFrWc5crm7yJSUJiMWpqoXL37ykWgk6Str2yRd519MogoRKoLfkCuRT33oY/0/*))#z6qy6pcq\",\"internal\":null,\"public\":\"wsh(sortedmulti(2,tpubDFW5Fj7gKnXEWErNAkhXqnKGjYupEoaP5674fynhFxio3FLWspZzWeMzquGXpVs1LknsbBvxPVu8XPWXZDwKFFr56kagYiPL8mLCiZ5Wq1K/0/*,[011aed20/44'/0'/0'/0]tpubDFZR9wnXt7a1fUVj2jEJw7E37pju124qjEfDDG9efozibH7T5BYtjwbjHtxHDK9q2nqsTYn25CngM2XUpXd1oZWCByWjkCG1YY1wnRvPTb8/0/*,[83488a89/44'/0'/0'/0]tpubDFi8BjbFcmJ3owMcf6SP2ZwZfYFWqi1C9PUv3EdorHCeipRycFrWc5crm7yJSUJiMWpqoXL37ykWgk6Str2yRd519MogoRKoLfkCuRT33oY/0/*))#6365az6x\"},\"address_look_ahead\":1,\"db_path\":null,\"server_uri\":null}
").unwrap().into();
        let jose_wallet: ElectrumMemoryWallet = serde_json::from_str::<WalletCfg>(" {\"name\":\"jose_wallet\",\"descriptors\":{\"network\":\"testnet\",\"external\":\"wsh(sortedmulti(2,[74d72b43/44'/0'/0'/0]tpubDFW5Fj7gKnXEWErNAkhXqnKGjYupEoaP5674fynhFxio3FLWspZzWeMzquGXpVs1LknsbBvxPVu8XPWXZDwKFFr56kagYiPL8mLCiZ5Wq1K/0/*,tprv8isP1XkHjjtLn1Tw95ZiXhZvYoDxqgsw9w4Rvk7MFYCKknrgSnjJZSys7k4aMx5Fk6vidUv7KWgj5RBRVR1aLqqNzZV6gDgpJS7PMNpza1C/0/*,[83488a89/44'/0'/0'/0]tpubDFi8BjbFcmJ3owMcf6SP2ZwZfYFWqi1C9PUv3EdorHCeipRycFrWc5crm7yJSUJiMWpqoXL37ykWgk6Str2yRd519MogoRKoLfkCuRT33oY/0/*))#yfgnjyt7\",\"internal\":null,\"public\":\"wsh(sortedmulti(2,[74d72b43/44'/0'/0'/0]tpubDFW5Fj7gKnXEWErNAkhXqnKGjYupEoaP5674fynhFxio3FLWspZzWeMzquGXpVs1LknsbBvxPVu8XPWXZDwKFFr56kagYiPL8mLCiZ5Wq1K/0/*,tpubDFZR9wnXt7a1fUVj2jEJw7E37pju124qjEfDDG9efozibH7T5BYtjwbjHtxHDK9q2nqsTYn25CngM2XUpXd1oZWCByWjkCG1YY1wnRvPTb8/0/*,[83488a89/44'/0'/0'/0]tpubDFi8BjbFcmJ3owMcf6SP2ZwZfYFWqi1C9PUv3EdorHCeipRycFrWc5crm7yJSUJiMWpqoXL37ykWgk6Str2yRd519MogoRKoLfkCuRT33oY/0/*))#ulfgmk2l\"},\"address_look_ahead\":1,\"db_path\":null,\"server_uri\":null}
").unwrap().into();

        let ahmed_wallet: ElectrumMemoryWallet = serde_json::from_str::<WalletCfg>(" {\"name\":\"ahmed_wallet\",\"descriptors\":{\"network\":\"testnet\",\"external\":\"wsh(sortedmulti(2,[74d72b43/44'/0'/0'/0]tpubDFW5Fj7gKnXEWErNAkhXqnKGjYupEoaP5674fynhFxio3FLWspZzWeMzquGXpVs1LknsbBvxPVu8XPWXZDwKFFr56kagYiPL8mLCiZ5Wq1K/0/*,[011aed20/44'/0'/0'/0]tpubDFZR9wnXt7a1fUVj2jEJw7E37pju124qjEfDDG9efozibH7T5BYtjwbjHtxHDK9q2nqsTYn25CngM2XUpXd1oZWCByWjkCG1YY1wnRvPTb8/0/*,tprv8j263KZ1UPcNvUKpmSmndAHT6WjagNpHa5t8kibWS1QFtLBCys2vRazzayhbAKNj9WdGaAC6gFjga7YibMNhboLeJKgfxS5b7ESy6gTMkAk/0/*))#zzx570uh\",\"internal\":null,\"public\":\"wsh(sortedmulti(2,[74d72b43/44'/0'/0'/0]tpubDFW5Fj7gKnXEWErNAkhXqnKGjYupEoaP5674fynhFxio3FLWspZzWeMzquGXpVs1LknsbBvxPVu8XPWXZDwKFFr56kagYiPL8mLCiZ5Wq1K/0/*,[011aed20/44'/0'/0'/0]tpubDFZR9wnXt7a1fUVj2jEJw7E37pju124qjEfDDG9efozibH7T5BYtjwbjHtxHDK9q2nqsTYn25CngM2XUpXd1oZWCByWjkCG1YY1wnRvPTb8/0/*,tpubDFi8BjbFcmJ3owMcf6SP2ZwZfYFWqi1C9PUv3EdorHCeipRycFrWc5crm7yJSUJiMWpqoXL37ykWgk6Str2yRd519MogoRKoLfkCuRT33oY/0/*))#zskvcukm\"},\"address_look_ahead\":1,\"db_path\":null,\"server_uri\":null}
").unwrap().into();

        adriana_wallet.sync(SifirWallet {}, Some(100)).unwrap();
        jose_wallet.sync(SifirWallet {}, Some(100)).unwrap();
        ahmed_wallet.sync(SifirWallet {}, Some(100)).unwrap();

        let adriana_address = adriana_wallet
            .get_address(AddressIndex::LastUnused)
            .unwrap();
        let jose_address = jose_wallet.get_address(AddressIndex::LastUnused).unwrap();
        let ahmed_addres = ahmed_wallet.get_address(AddressIndex::LastUnused).unwrap();

        assert_eq!(adriana_address, jose_address);
        assert_eq!(jose_address, ahmed_addres);
        //
        // Send some sats to this scripts address
        //println!("next address: {}",adriana_address );

        adriana_wallet.sync(SifirWallet {}, Some(100)).unwrap();
        jose_wallet.sync(SifirWallet {}, Some(100)).unwrap();
        ahmed_wallet.sync(SifirWallet {}, Some(100)).unwrap();

        let adoni_balance = adriana_wallet.get_balance().unwrap();
        let janis_balance = jose_wallet.get_balance().unwrap();
        let albert_balance = ahmed_wallet.get_balance().unwrap();

        //println!( "wallet balance {}",   adoni_balance );
        assert!(adoni_balance > 0);
        assert_eq!(adoni_balance, janis_balance);
        assert_eq!(janis_balance, albert_balance);

        let mut txn = jose_wallet.build_tx();
        txn.set_single_recipient(
            Address::from_str(rcvr_address)
                .unwrap()
                .payload
                .script_pubkey(),
        )
        .drain_wallet()
        .fee_rate(FeeRate::from_sat_per_vb(1.0))
        .enable_rbf();

        let (mut psbt, _tx_details) = txn.finish().unwrap();
        let finished = jose_wallet.sign(&mut psbt, SignOptions::default()).unwrap();
        assert!(!finished);
        let finished = ahmed_wallet
            .sign(&mut psbt, SignOptions::default())
            .unwrap();
        assert!(finished);

        //let txn_id = jose_wallet.broadcast(psbt.extract_tx()).unwrap();
        //println!("txnId: {}",txn_id);
        //assert_eq!(txn_id.len(),32)
    }
}
