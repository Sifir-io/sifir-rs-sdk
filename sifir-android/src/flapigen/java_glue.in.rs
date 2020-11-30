use jni_sys::*;
use tor::{OwnedTorService, TorServiceParam,OwnedTorServiceBootstrapPhase,BootstrapPhase};
use serde::{Serialize};

foreign_class!(class TorServiceParam {
    self_type TorServiceParam;
    constructor TorServiceParam::new(data_dir:&str,socks_port:u16)->TorServiceParam;
});
/// This is what Java will be calling
foreign_class!(class OwnedTorService {
    self_type OwnedTorService;
    constructor OwnedTorService::new(param:TorServiceParam)->OwnedTorService;
    fn getSocksPort(&self)-> u16{
        this.socks_port
    }
    fn OwnedTorService::shutdown(&mut self);
    fn get_status(&mut self)-> String{
        let node_status = this.get_status();
        match node_status {
        Ok(status) => {
            let status_string = serde_json::to_string(&status).unwrap();
            println!("status is {}", status_string);
            status_string
        }
        Err(e) => {
            let message = match e.downcast::<String>() {
                Ok(msg) => msg,
                Err(_) => String::from("Unknown error"),
            };
            message
        }

    }}}

);
