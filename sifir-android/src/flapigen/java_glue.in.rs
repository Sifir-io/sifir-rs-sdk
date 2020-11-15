use tor::{OwnedTorService,TorService, TorServiceParam};
use jni::*;
use tokio::macros::support::Future;
use tokio::net::TcpStream;
use torut::control::{AsyncEvent, AuthenticatedConn, ConnError, UnauthenticatedConn};
use crate::utils::{CallBackResult,CallBack};

//pub enum CallBackResult {
//    Success(String),
//    Error(String),
//}
//pub trait CallBack {
//    fn on_state_changed(&self, result: CallBackResult);
//}
//
//foreign_callback!(callback CallBack {
//    self_type CallBack;
//    onStateUpdate = CallBack::on_state_changed(&self, item: CallBackResult);
//});


foreign_class!(class TorServiceParam {
    self_type TorServiceParam;
    private constructor = empty;
    fn getDataDir(&self) -> &str {
        &this.data_dir
     }
    fn getSockPort(&self) -> &str {
        &this.socks_port
    }
    fn getPort(&self) -> &str {
        &this.port
    }
});
/// This is what Java will be calling
foreign_class!(class OwnedTorService {
    self_type OwnedTorService;
    // FIXME this side of the code needs to know socks port at least to create its client
    constructor OwnedTorService::new(param:TorServiceParam)->OwnedTorService;
    fn OwnedTorService::shut_down(&mut self);
    // TODO add create_hidden_Service
});
