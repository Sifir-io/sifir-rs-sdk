use anyhow::Result;
use serde::{Deserialize, Serialize};
// use std::cell::RefCell;
// use std::future::Future;
use libtor::{HiddenServiceVersion, Tor, TorAddress, TorFlag};
use std::thread::JoinHandle;
use tokio::macros::support::Future;
use tokio::net::TcpStream;
use torut::control::{AsyncEvent, AuthenticatedConn, ConnError, UnauthenticatedConn};

#[derive(Serialize, Deserialize, Debug)]
pub enum TorRequestMethod {
    Get,
    Post,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TorRequest {
    method: TorRequestMethod,
    payload: String,
    url: String,
    signature_header: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TorServiceParam {
    port: u16,
    socks_port: Option<u16>,
    data_dir: String,
}

pub struct TorService {
    socks_port: u16,
    control_port: u16,
    _handle: Option<JoinHandle<Result<u8, libtor::Error>>>,
}
impl From<TorServiceParam> for TorService {
    fn from(param: TorServiceParam) -> Self {
        let mut service = Tor::new();
        let socks_port = param.socks_port.unwrap_or(19051);
        let control_port = 19052;
        service
            .flag(TorFlag::DataDirectory(param.data_dir.clone()))
            .flag(TorFlag::SocksPort(socks_port))
            // .flag(TorFlag::TestSocks(libtor::TorBool::True))
            // .flag(TorFlag::ControlPortAuto)
            .flag(TorFlag::ControlPort(control_port))
            .flag(TorFlag::CookieAuthentication(libtor::TorBool::True))
            .flag(TorFlag::ControlPortWriteToFile(format!(
                "{}/ctl.info",
                param.data_dir.clone()
            )))
            .flag(TorFlag::ControlPortFileGroupReadable(libtor::TorBool::True));

        let handle = service.start_background();

        // FIXME go get the config file from the dir once ConfigAuto is fixed
        // save it control port etc info

        TorService {
            socks_port,
            control_port,
            _handle: Some(handle),
        }
    }
}

// How can i do tthis one too ?, H,S ?
//impl From<TorServiceParam> for AuthenticatedConn<H,S> {
//    fn from(param: TorServiceParam) -> Self {
//        // FIXME how cani do this ?
//        async {
//            let s = TcpStream::connect(&format!("127.0.0.1:{}", self.control_port))
//                .await
//                .unwrap();
//            let mut utc = UnauthenticatedConn::new(s);
//            // returns node info + cookie location ?
//            let proto_info = utc.load_protocol_info().await.unwrap();
//            // loads cookie from loaded data and build auth info
//            let auth = proto_info.make_auth_data().unwrap().unwrap();
//            utc.authenticate(&auth).await.unwrap();
//            // upgrade connection to authenticated
//            let mut ac = utc.into_authenticated().await;
//            ac.set_async_event_handler(Some(handle));
//            *self._ctl.borrow_mut() = Some(ac);
//        }
//    }
//}

// impl TorService {
// pub fn new() -> TorService {
impl TorService {
    pub fn get_client(&self) -> Result<reqwest::Client, reqwest::Error> {
        let proxy = reqwest::Proxy::all(
            reqwest::Url::parse(format!("socks5h://127.0.0.1:{}", self.socks_port).as_str())
                .unwrap(),
        )
        .unwrap();
        reqwest::Client::builder().proxy(proxy).build()
    }
    //async fn get_control_auth_conn(&self) -> AuthenticatedConn<TcpStream, Option<None>> {
    //    let s = TcpStream::connect(&format!("127.0.0.1:{}", self.control_port))
    //        .await
    //        .unwrap();
    //    let mut utc = UnauthenticatedConn::new(s);
    //    // returns node info + cookie location ?
    //    let proto_info = utc.load_protocol_info().await.unwrap();
    //    // loads cookie from loaded data and build auth info
    //    let auth = proto_info.make_auth_data().unwrap().unwrap();
    //    utc.authenticate(&auth).await.unwrap();
    //    // upgrade connection to authenticated
    //    let mut ac = utc.into_authenticated().await;
    //    ac
    //}
}
//
//impl<F, I> TorService
//where
//    F: Fn(AsyncEvent<'static>) -> I + 'static,
//    I: Future<Output = Result<(),ConnError>>,
//{
//    pub fn stop_service(&self) {
//        match &self._handle {
//            Some(x) => {
//                x.join().expect("Error waiting for service thread");
//            }
//            None => {
//                println!("No service detected");
//            }
//        }
//    }
//    pub async fn get_bootstarp_phase(&self) -> String {
//        let mut ctl = self._ctl.borrow_mut();
//        ctl.as_mut()
//            .unwrap()
//            .get_info("status/bootstrap-phase")
//            .await
//            .unwrap()
//    }
//    pub async fn take_ownership(&self) {
//        let mut ctl = self._ctl.borrow_mut();
//        ctl.as_mut().unwrap().take_ownership().await.unwrap()
//    }
//}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn should_get_service_from_param_and_get_an_onion() {
        let service: TorService = TorServiceParam {
            port: 8000,
            socks_port: Some(19051),
            data_dir: String::from("/tmp/torlib"),
        }
        .into();
        assert_eq!(service.socks_port, 19051);
        assert_eq!(service._handle.is_some(), true);
        let client = service.get_client().unwrap();
        let resp = client
            .get("http://keybase5wmilwokqirssclfnsqrjdsi7jdir5wy7y7iu3tanwmtp6oid.onion")
            .send()
            .await
            .unwrap();
        println!("{:#?}", resp);
        assert_eq!(resp.status(), 200);
        let _ = service._handle.unwrap().join();
    }

    // Tell bootstraping is done
    // let info = service.get_bootstarp_phase().await;
    // assert_eq!(info.contains("PROGRESS=100 TAG=done"), true)
    //}
}
