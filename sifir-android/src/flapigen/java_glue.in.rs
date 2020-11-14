use crate::tor::{TorService, TorServiceParam};
use jni_sys::*;
use tokio::macros::support::Future;
use tokio::net::TcpStream;
use torut::control::{AsyncEvent, AuthenticatedConn, ConnError, UnauthenticatedConn};
use crate::utils::{CallBackResult,CallBack};

pub enum CallBackResult {
    Success(String),
    Error(String),
}
pub trait CallBack {
    fn on_state_changed(&self, result: CallBackResult);
}

foreign_callback!(callback CallBack {
    self_type CallBack;
    onStateUpdate = CallBack::on_state_changed(&self, item: CallBackResult);
});


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

foreign_class!(class TorService {
    self_type TorService;
    constructor Tor::new()->TorService;
    fn TorService::start_service<T>(&self, handle: Option<H>) -> AuthenticatedConn<TcpStream, H>;
});
