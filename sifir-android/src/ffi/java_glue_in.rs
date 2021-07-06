use btc::*;
use jni_sys::*;
use logger::{log, Logger};
use serde::Serialize;
use std::time::Duration;
use tor::{
    hidden_service::{HiddenServiceDataHandler, HiddenServiceHandler},
    tcp_stream::{DataObserver, TcpSocksStream},
    BootstrapPhase, OwnedTorService, OwnedTorServiceBootstrapPhase, TorHiddenService,
    TorHiddenServiceParam, TorServiceParam,
};
use serde_json::json;

/// Java callback interface for DataObserver callback used in TcpStreams, HiddenService etc..
foreign_callback!(callback DataObserver {
    self_type DataObserver;
    onData = DataObserver::on_data(&self,result: String);
    onError = DataObserver::on_error(&self,result: String);
});
// internally wrap passed the Boxed DataObserver Impl we receive from Java
// with Observer so we can Send across threads
unsafe impl Send for Observer {}

unsafe impl Sync for Observer {}

struct Observer {
    cb: Box<dyn DataObserver>,
}

impl DataObserver for Observer {
    fn on_data(&self, data: String) {
        self.cb.on_data(data);
    }
    fn on_error(&self, data: String) {
        self.cb.on_error(data);
    }
}

/// Hiden Service Handler
#[cfg(feature = "tor_daemon")]
foreign_class!(class HiddenServiceHandler {
    self_type HiddenServiceHandler;
    constructor new(dst_port:u16,cb:Box<dyn DataObserver>)->Result<HiddenServiceHandler,String>{
       let mut lsnr = HiddenServiceHandler::new(dst_port).map_err(|e| { format!("{:#?}",e) }).unwrap();
       lsnr.set_data_handler(Observer { cb }).map_err(|e| { format!("{:#?}",e) }).unwrap();
       let _ = lsnr.start_http_listener();
       Ok(lsnr)
    }
});

/// Tor Hidden Service, cannot be constructed directly
#[cfg(feature = "tor_daemon")]
foreign_class!(class TorHiddenService {
    self_type TorHiddenService;
    private constructor = empty;
    fn get_onion_url(&self)->String{
        this.onion_url.to_string()
    }
    fn get_secret_b64(&self)->String{
        base64::encode(this.secret_key).into()
    }
});

/// OwnedTorService Android Interface
#[cfg(feature = "tor_daemon")]
foreign_class!(class TorServiceParam {
    self_type TorServiceParam;
    constructor TorServiceParam::new(data_dir:&str,socks_port:u16,bootstap_timeout_ms: u64)->TorServiceParam;
});

#[cfg(feature = "tor_daemon")]
foreign_class!(class OwnedTorService {
    self_type OwnedTorService;
    constructor new(param:TorServiceParam)->Result<OwnedTorService,String> {
        Logger::new();
        OwnedTorService::new(param).map_err(|e| { format!("{:#?}",e)})
    }
    fn getSocksPort(&self)-> u16{
        this.socks_port
    }
    fn shutdown(&mut self)->Result<(),String>{
        this.shutdown().map_err(|e| { format!("{:#?}",e) })
    }
    fn get_status(&mut self)-> String {
        let node_status = this.get_status();
        match node_status {
            Ok(status) => {
                let status_string = serde_json::to_string(&status).unwrap();
                status_string
            }
            Err(e) => { e.to_string() }
       }
    }
    fn create_hidden_service(&mut self,  dst_port: u16, hs_port: u16, secret_key: String) -> Result<TorHiddenService,String> {
        let hs_key = match secret_key.len() {
            0 => Ok(None),
            _ => {
                let mut decoded_buff:[u8;64] = [0;64];
                base64::decode_config_slice(secret_key, base64::STANDARD, &mut decoded_buff)
                    .map( |_| { Some(decoded_buff)})
            }
        };

        match hs_key {
            Ok(key) =>{
                this.create_hidden_service(TorHiddenServiceParam {
                    to_port: dst_port,
                    hs_port,
                    secret_key: key
                }).map_err(|e| { format! ("{:#?}",e)})
            },
            Err(e)=> {
              Err(format!("{:#?}",e))
            }
          }
    }
    fn delete_hidden_service(&mut self,  onion: String) -> Result<(),String> {
         this.delete_hidden_service(onion).map_err(|e| { format! ("{:#?}",e)})
    }
});

/// TcpStream Android Interface
#[cfg(feature = "tor_daemon")]
foreign_class!(class TcpSocksStream {
    self_type TcpSocksStream;
    constructor new(target:String,socks_proxy:String,timeout_ms:u64)->Result<TcpSocksStream,String> {
      TcpSocksStream::new_timeout(target,socks_proxy,timeout_ms).map_err(|e| { format!("{:#?}",e) })
    }
    fn on_data(&mut self,cb:Box<dyn DataObserver>)->Result<(),String>{
      this.set_data_handler(Observer{
       cb,
      }).unwrap();
      this.read_line_async().map_err(|e| { format!("{:#?}",e)})
    }
    fn send_data(&mut self, msg:String,timeout:u64)->Result<(),String>{
        this.send_data(msg, Some(Duration::new(timeout, 0))).map_err(|e| { format!("{:#?}",e) })
    }
});
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
                println!(
                    "android ffi sync progress is {} and message {:?}, TODO THIS TO OBSERVER",
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
