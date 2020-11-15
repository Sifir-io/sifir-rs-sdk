use jni_sys::*;
use utils::{CallBackResult,CallBack};
use tor::{OwnedTorService,TorService, TorServiceParam};

foreign_class!(class TorServiceParam {
    self_type TorServiceParam;
    private constructor = empty;
});
/// This is what Java will be calling
foreign_class!(class OwnedTorService {
    self_type OwnedTorService;
    constructor OwnedTorService::new(param:TorServiceParam)->OwnedTorService;
    fn OwnedTorService::shutdown(&mut self);
    // TODO add create_hidden_Service
});
