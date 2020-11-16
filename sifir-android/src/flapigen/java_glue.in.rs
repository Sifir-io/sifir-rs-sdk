use jni_sys::*;
// use utils::{CallBackResult,CallBack};
use tor::{OwnedTorService, TorServiceParam};

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
});
