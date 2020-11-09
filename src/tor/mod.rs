#[macro_use]
use libtor::{HiddenServiceVersion, Tor, TorAddress, TorFlag};
use anyhow::Result;
use std::cell::RefCell;
use std::future::Future;
use std::thread::JoinHandle;
use tokio::net::TcpStream;
use torut::control::{AsyncEvent, AuthenticatedConn, ConnError, UnauthenticatedConn};

enum TorRequestMethod {
    Get,
    Post,
}
struct TorRequest {
    method: TorRequestMethod,
    payload: String,
    url: String,
    signature_header: String,
}

// FIXME this??
//https://stackoverflow.com/questions/58354633/cannot-use-impl-future-to-store-async-function-in-a-vector
type Fut = Box<dyn Future<Output=()>>;
type RER = dyn Fn(AsyncEvent<'static>)-> Fut + 'static;
type TorutController = AuthenticatedConn<TcpStream, Box<RER>>;


// async fn handle_tor_async(event: AsyncEvent<'static>)-> Future<Output=Result()>{
//     println!("async handler got {:?}", event);
//     async move {
//         Ok(())
//     }
// }

// pub struct TorService
pub struct TorService {
    port: u16,
    socks_port: u16,
    control_port: u16,
    data_dir: String,
    _handle: Option<JoinHandle<Result<u8, libtor::Error>>>,
    _ctl: RefCell<Option<TorutController>>,
}

// impl TorService {
// pub fn new() -> TorService {
impl TorService {
    pub fn new() -> TorService {
        TorService {
            port: 8000,
            socks_port: 19051,
            control_port: 19052,
            data_dir: String::from("/tmp/tor-rust"),
            _handle: None,
            _ctl: RefCell::new(None),
        }
    }
    pub fn start_service(&mut self) -> Result<()> {
        // TODO check if we already have a handle/ctl connection then error out
        let data_dir = &self.data_dir;
        let mut service = Tor::new();
        service
            .flag(TorFlag::DataDirectory(data_dir.into()))
            .flag(TorFlag::SocksPort(self.socks_port))
            .flag(TorFlag::TestSocks(libtor::TorBool::True))
            .flag(TorFlag::ControlPortAuto)
            // .flag(TorFlag::ControlPort(self.control_port))
            .flag(TorFlag::CookieAuthentication(libtor::TorBool::True))
            .flag(TorFlag::ControlPortWriteToFile(format!(
                "{}/ctl.info",
                data_dir
            )))
            .flag(TorFlag::ControlPortFileGroupReadable(libtor::TorBool::True));

        let handle = service.start_background();
        self._handle = Some(handle);
        self.get_control_handle(Box::new(|event|{
                                    async move {
                                        println!("{:?}",event);
                                        Ok(())
                                    }
                               }));
        // TODO loop until we get 100% boostrap
        println!("Sleeping..");
        let ten_millis = std::time::Duration::from_secs(7);
        std::thread::sleep(ten_millis);
        Ok(())
    }

    pub async fn get_control_handle(&mut self,handle:Box<RER>)
            {
        let s = TcpStream::connect(&format!("127.0.0.1:{}", self.control_port))
            .await
            .unwrap();
        let mut utc = UnauthenticatedConn::new(s);
        // returns node info + cookie location ?
        let proto_info = utc.load_protocol_info().await.unwrap();
        // loads cookie from loaded data and build auth info
        let auth = proto_info.make_auth_data().unwrap().unwrap();
        utc.authenticate(&auth).await.unwrap();
        // upgrade connection to authenticated
        let mut ac = utc.into_authenticated().await;
        ac.set_async_event_handler(Some(handle));
        *self._ctl.borrow_mut() = Some(ac);
    }
    pub fn get_client(&self) -> Result<reqwest::Client, reqwest::Error> {
        let proxy = reqwest::Proxy::all(
            reqwest::Url::parse(format!("socks5h://127.0.0.1:{}", self.socks_port).as_str())
                .unwrap(),
        )
        .unwrap();
        reqwest::Client::builder().proxy(proxy).build()
    }
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
    //lazy_static! {
    //  static ref tor_service: TorService = TorService::new();
    //}
    use super::*;

    #[tokio::test]
    async fn should_be_able_to_get_a_client_and_GET_onion() {
        println!("setting sevice");
        let mut service = TorService::new();
        service.start_service();
        // let service = &*tor_service;
        println!("service set copmlete");
        let client = service.get_client().unwrap();
        let resp = client
            .get("http://keybase5wmilwokqirssclfnsqrjdsi7jdir5wy7y7iu3tanwmtp6oid.onion")
            .send()
            .await
            .unwrap();
        println!("Call copmlete");
        println!("{:#?}", resp);
        assert_eq!(resp.status(), 200);
        // }

        // Tell bootstraping is done
        // let info = service.get_bootstarp_phase().await;
        // assert_eq!(info.contains("PROGRESS=100 TAG=done"), true)
    }
}
