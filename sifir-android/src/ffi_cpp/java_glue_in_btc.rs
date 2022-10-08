#[cfg(feature = "btc_wallet")]
use btc::*;
use bdk::bitcoin::consensus::encode::{deserialize, serialize};
use bdk::bitcoin::util::psbt::PartiallySignedTransaction;
use bdk::SignOptions;

macro_rules! unwrap_err_string {
    ($e:expr)=>{
        $e.map_err(|e|{ format!("{:#?}",e)}).unwrap();
    }
}

// Generate Xprvs
#[cfg(feature = "btc_wallet")]
foreign_class!(class Keys{
    fn derive_xprvs(network: String, derive_path: String, password: String, mnemonic:String, num_child: u32)->Result<String,String> {
        // FIXME this shouold be impl From<str> for Network {
        //
        // }
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
        let keys = DerivedBip39Xprvs::new(
            derive_path.into_derivation_path().unwrap(),
            network,
            num_child,
            Some(String::from(password)),
            match mnemonic.len() {
                x if x > 0 => Some(String::from(mnemonic)),
                _ => None,
            },
        )
        .map_err(|e|{ format!("{:#?}",e)}).unwrap();
        serde_json::to_string(&keys).map_err(|e|{ format!("{:#?}",e)})
    }
fn xprvs_w_paths_to_xpubs_w_paths(
    xprvspaths_vector_json_string: String,
    network_str: String
) -> Result<String,String>{
        let network = match network_str.as_str() {
            "testnet" => Ok(Network::Testnet),
            "mainnet" => Ok(Network::Bitcoin),
            "bitcoin" => Ok(Network::Bitcoin),
            _ => Err("Invalid network passed"),
        }
        .unwrap();
        let xpubs_with_paths: XpubsWithPaths = (
            unwrap_err_string!(serde_json::from_str::<XprvsWithPaths>(&xprvspaths_vector_json_string)),
            network
        )
            .into();
        Ok(unwrap_err_string!(serde_json::to_string(&xpubs_with_paths)))
}

});
/// Xprvs To Descriptors
#[cfg(feature = "btc_wallet")]
foreign_class!(class Descriptors{
    fn wallet_descriptors_from_any_descriptor_cfg(any_desc_cfg: String)->Result<String,String> {

        let wallet_descriptors: WalletDescriptors =
        unwrap_err_string!(serde_json::from_str::<AnyDescriptorCfg>(&any_desc_cfg)).into();
        // TODO: this should not be unwraped, thus no use of macro
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
    fn get_new_address(&mut self,index:u32)->Result<String,String>{
        this.get_address(match index {
                            0 => AddressIndex::LastUnused,
                                            1 => AddressIndex::New,
                                                            _ => AddressIndex::Peek(index),
                                                                        }).map_err(|e| { format!("{:#?}",e)}).map(|address| format!("{}",address))
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
        let _ = unwrap_err_string!(this.sync(SifirWallet {}, Some(max_address_count)));
        Ok(())
    }
    fn create_tx(&mut self,tx: String)-> Result<String,String> {
        let create_txn: CreateTx = unwrap_err_string!(serde_json::from_str(&tx));
        let (psbt, txn) = unwrap_err_string!(create_txn.into_wallet_txn(this));
        Ok(json!({"psbt": base64::encode(serialize(&psbt)), "txnDetails" : txn}).to_string())
    }
    fn sign_psbt(&mut self,psbt_base64:String) -> Result<String,String>{
        let mut psbt = deserialize(&base64::decode(&psbt_base64).unwrap()).unwrap();
        let finished = unwrap_err_string!(this.sign(&mut psbt, SignOptions::default()));
        Ok(
         json!({"psbt" : base64::encode(serialize(&psbt)).to_string(), "finished": finished}).to_string())
    }
    fn broadcast_pbst(&mut self, psbt_base64: String) -> Result<String,String>{
              let psbt = deserialize::<PartiallySignedTransaction>(&base64::decode(&psbt_base64).unwrap()).unwrap();
            let txn_id = this.broadcast(psbt.extract_tx()).unwrap();
            Ok(txn_id.to_string())
    }

});
// FIXME 
// 1. HERE map_err macro to rest of stuff
// 2. deserialize psbt in a util class as static ?
