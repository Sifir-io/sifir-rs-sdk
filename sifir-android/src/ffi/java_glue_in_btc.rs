#[cfg(feature = "btc_wallet")]
use btc::*;

// Generate Xprvs
#[cfg(feature = "btc_wallet")]
foreign_class!(class DerivedBip39Xprvs{
    fn derive_xprvs(network: String, derive_path: String, password: String, mnemonic:String, num_child: usize)->Result<String,String> {
        let network = match network.as_str() {
            "testnet" => Ok(Network::Testnet),
            "mainnet" => Ok(Network::Bitcoin),
            "bitcoin" => Ok(Network::Bitcoin),
            _ => Err("Invalid network passed"),
        }.unwrap();
        let num_child = match num_child {
            x if x >= 2 => x,
            _ => 2,
        };
        let wallet_desc = DerivedBip39Xprvs::new(
            derive_path.into_derivation_path().map_err(|e| { format!("{:#?}",e)}).unwrap(),
            network,
            num_child as u32,
            Some(password),
            match mnemonic.len() {
                x if x > 0 => Some(mnemonic),
                _ => None,
            },
        ).map_err(|e|{ format!("{:#?}",e)}).unwrap();
        serde_json::to_string(&wallet_desc).map_err(|e|{ format!("{:#?}",e)})
}});
/// Xprvs To Descriptors
#[cfg(feature = "btc_wallet")]
foreign_class!(class DerivedBip39Xprvs{
    fn descriptors_from_xprvs_wpaths_vec(vec_xprvs_with_paths_json: String, network: String)->Result<String,String> {
        let network = match network.as_str() {
            "testnet" => Ok(Network::Testnet),
            "mainnet" => Ok(Network::Bitcoin),
            "bitcoin" => Ok(Network::Bitcoin),
            _ => Err("Invalid network passed"),
        }
        .unwrap();
        let x_prvs_with_path: Vec<XprvsWithPaths> = serde_json::from_str(&vec_xprvs_with_paths_json).map_err(|e|{ format!("{:#?}",e)}).unwrap();
        let wallet_descriptors: WalletDescriptors = (x_prvs_with_path, network).into();
        serde_json::to_string(&wallet_descriptors).map_err(|e|{ format!("{:#?}",e)})
}});
/// OwnedTorService Android Interface
#[cfg(feature = "btc_wallet")]
foreign_class!(class ElectrumSledWallet {
    self_type ElectrumSledWallet;
    constructor new(wallet_cfg_json:String)->Result<ElectrumSledWallet,String> {
        let wallet_cfg: WalletCfg = serde_json::from_str(&wallet_cfg_json).map_err(|e| { format!("{:#?}",e)}).unwrap();
        Ok(Into::<ElectrumSledWallet>::into(wallet_cfg))
    }
    fn get_balance(&self)-> Result<u64,String>{
        this.get_balance().map_err(|e| { format!("{:#?}",e)})
    }
    fn get_new_address(&mut self)->Result<String,String>{
        this.get_address(AddressIndex::New).map_err(|e| { format!("{:#?}",e)}).map(|address| format!("{}",address))
    }
    fn sync(&mut self,max_address_count:u32)-> Result<(),String> {
        struct SifirWallet {};
        impl Progress for SifirWallet {
            fn update(&self, progress: f32, message: Option<String>) -> Result<(), bdk::Error> {
                info!(
                    "Wallet sync progress is {} and message {:?}, TODO THIS TO OBSERVER",
                    progress, message
                );
                Ok(())
            }
        };
        let _ = this.sync(SifirWallet {}, Some(max_address_count)).map_err(|e| { format!("{:#?}",e)}).unwrap();
        Ok(())
    }
    fn create_tx(&mut self,tx: String)-> Result<String,String> {
        let create_txn: CreateTx = serde_json::from_str(&tx).unwrap();
        let (pp, txn) = create_txn.into_wallet_txn(this).unwrap();
        Ok(json!({"partiallySignedPsbt": pp, "txnDetails" : txn}).to_string())
    }
});
