// use crate::utils;
use anyhow::Result;
use futures::Future;
use lazy_static::*;
use libtor::{HiddenServiceVersion, Tor, TorAddress, TorFlag};
use serde::{Deserialize, Serialize};
#[cfg(test)]
use serial_test::serial;
use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::fs;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::pin::Pin;
use std::sync::Mutex;
use std::thread::JoinHandle;
use tokio::net::TcpStream;
use torut::control::{AsyncEvent, AuthenticatedConn, ConnError, UnauthenticatedConn};
use torut::onion::TorSecretKeyV3;
use utils::CallBack;

type F = Box<dyn Fn(AsyncEvent<'static>) -> Pin<Box<dyn Future<Output = Result<(), ConnError>>>>>;
type G = AuthenticatedConn<TcpStream, F>;
lazy_static! {
    static ref RUNTIME: Mutex<tokio::runtime::Runtime> = Mutex::new(
        tokio::runtime::Builder::new()
            .threaded_scheduler()
            .enable_all()
            .build()
            .unwrap()
    );
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TorServiceParam {
    pub socks_port: Option<u16>,
    pub data_dir: String,
}
impl TorServiceParam {
    /// A constructor for TorServiceParam to make it easier to be called from 
    /// an FFI 
    pub fn new(data_dir:&str,socks_port:u16)->TorServiceParam{
        TorServiceParam{
            data_dir: String::from(data_dir),
            socks_port: Some(socks_port)
        }
    }
}

pub struct TorService {
    socks_port: u16,
    control_port: String,
    _handle: Option<JoinHandle<Result<u8, libtor::Error>>>,
}

pub struct OwnedTorService {
    pub socks_port: u16,
    pub control_port: String,
    _handle: Option<JoinHandle<Result<u8, libtor::Error>>>,
    _ctl: RefCell<Option<G>>,
}

pub struct TorHiddenServiceParam {
    port: u16,
}

pub struct TorHiddenService {
    onion_url: TorAddress,
    secret_key: [u8; 64],
}

//trait TorSocksProxy {
//    fn get_socks_port(&self) -> u16;
//}

trait TorControlApi {
    // async fns in traits are a shitshow
    fn wait_bootstrap(&mut self) -> Pin<Box<dyn Future<Output = Result<bool, ()>> + '_>>;
    fn shutdown(self);
}

impl From<TorServiceParam> for TorService {
    fn from(param: TorServiceParam) -> Self {
        let mut service = Tor::new();
        let socks_port = param.socks_port.unwrap_or(19051);
        service
            .flag(TorFlag::DataDirectory(param.data_dir.clone()))
            .flag(TorFlag::SocksPort(socks_port))
            .flag(TorFlag::ControlPortAuto)
            .flag(TorFlag::CookieAuthentication(libtor::TorBool::True))
            .flag(TorFlag::ControlPortWriteToFile(format!(
                "{}/ctl.info",
                param.data_dir
            )))
            .flag(TorFlag::ControlPortFileGroupReadable(libtor::TorBool::True));

        let handle = service.start_background();

        let mut is_ready = false;
        let mut control_port = String::new();
        let mut try_times = 0;
        // TODO We wait for Tor to write the new config file otherwise we risk reading the old config and port.
        // Anyway to *know* when the new config has been written besides checking config file modifed after starting process?
        std::thread::sleep(std::time::Duration::from_millis(1000));
        while !is_ready {
            let contents = fs::read_to_string(format!("{}/ctl.info", param.data_dir.clone()));
            match contents {
                Ok(t) => {
                    assert!(t.contains("PORT="));
                    let data: Vec<&str> = t.split("PORT=").collect();
                    control_port = data[1].into();
                    is_ready = true;
                }
                Err(e) => {
                    try_times += 1;
                    if try_times > 7 {
                        panic!("Could load Tor control config file");
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(300));
        }

        TorService {
            socks_port,
            control_port,
            _handle: Some(handle),
        }
    }
}

fn handler(
    event: AsyncEvent<'static>,
) -> Pin<Box<dyn Future<Output = Result<(), ConnError>> + '_>> {
    Box::pin(async move { Ok(()) })
}

impl TorService {
    pub fn new(param: TorServiceParam) -> Self {
        param.into()
    }
    pub fn get_client(&self) -> Result<reqwest::Client, reqwest::Error> {
        let proxy = reqwest::Proxy::all(
            reqwest::Url::parse(format!("socks5h://127.0.0.1:{}", self.socks_port).as_str())
                .unwrap(),
        )
        .unwrap();
        reqwest::Client::builder().proxy(proxy).build()
    }
    async fn get_control_auth_conn<F>(&self, handle: Option<F>) -> AuthenticatedConn<TcpStream, F> {
        let s = TcpStream::connect(self.control_port.trim()).await.unwrap();
        let mut utc = UnauthenticatedConn::new(s);
        // returns node info + cookie location ?
        let proto_info = utc.load_protocol_info().await.unwrap();
        // loads cookie from loaded data and build auth info
        let auth = proto_info.make_auth_data().unwrap().unwrap();
        utc.authenticate(&auth).await.unwrap();
        // upgrade connection to authenticated
        let mut ac = utc.into_authenticated().await;
        if handle.is_some() {
            ac.set_async_event_handler(handle);
        }
        ac
    }
    // FIXME here accept callback trait for FFI
    /// Probably should be renamed to to_owned_node()
    /// Converts TorService to OwnedTorService, consuming the TorService
    pub fn to_owned_node(self, callback: Option<Box<CallBack>>) -> OwnedTorService {
        (*RUNTIME).lock().unwrap().block_on(async {
            let mut ac = self
                .get_control_auth_conn(Some(Box::new(handler) as F))
                .await;
            ac.wait_bootstrap().await.unwrap();
            ac.take_ownership().await.unwrap();
            OwnedTorService {
                socks_port: self.socks_port,
                control_port: self.control_port,
                _handle: self._handle,
                _ctl: RefCell::new(Some(ac)),
            }
        })
    }
}

impl From<TorServiceParam> for OwnedTorService {
    fn from(param: TorServiceParam) -> Self {
        let t: TorService = param.into();
        t.to_owned_node(None)
    }
}

/// Implemntation when TorService has AuthenticatedConnection established
/// This is what the FFI interacts with
impl OwnedTorService {
    pub fn new(param: TorServiceParam) -> Self {
        param.into()
    }
    // TODO check port is not already taken
    pub fn create_hidden_service(
        &mut self,
        param: TorHiddenServiceParam,
    ) -> Result<TorHiddenService, ConnError> {
        (*RUNTIME).lock().unwrap().block_on(async {
            let mut _ctl = self._ctl.borrow_mut();
            let ctl = _ctl.as_mut().unwrap();
            let service_key = TorSecretKeyV3::generate();
            ctl.add_onion_v3(
                &service_key,
                false,
                false,
                false,
                None,
                &mut [(
                    param.port,
                    SocketAddr::new(IpAddr::from(Ipv4Addr::new(127, 0, 0, 1)), param.port),
                )]
                .iter(),
            )
            .await
            .unwrap();
            let onion_url = TorAddress::AddressPort(
                service_key.public().get_onion_address().to_string(),
                param.port,
            );
            let secret_key = service_key.as_bytes();
            Ok(TorHiddenService {
                onion_url,
                secret_key,
            })
        })
    }

    /// take control conn and drop it.
    /// Closing the owned connection and causes tor daemon to shutdown
    /// Then waits on the Tor daemon thread to exit
    pub fn shutdown(&mut self) {
        {
            let _ = self._ctl.borrow_mut().take();
        }
        let _ = self._handle.take().unwrap().join();
    }
}

//impl TorSocksProxy for OwnedTorService {
//    fn get_socks_port(&self) -> u16 {
//        self.socks_port
//    }
//}

/// High level API for Torut used internally by TorService to expose
/// note control functions to FFI and user
impl<F, H> TorControlApi for AuthenticatedConn<TcpStream, H>
where
    H: Fn(AsyncEvent<'static>) -> F,
    F: Future<Output = Result<(), ConnError>>,
{
    fn wait_bootstrap(&mut self) -> Pin<Box<dyn Future<Output = Result<bool, ()>> + '_>> {
        // Wait for boostrap to be done
        Box::pin(async move {
            let mut input = String::new();
            while !input.trim().contains("PROGRESS=100 TAG=done") {
                input = self.get_info("status/bootstrap-phase").await.unwrap();
                std::thread::sleep(std::time::Duration::from_millis(700));
            }
            Ok(true)
        })
    }
    // dropping the control connection after having taken ownership of the node will cause the node
    // to shutdown
    fn shutdown(self) {}
}
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    #[serial(tor)]
    async fn get_from_param_and_await_boostrap_using_api() {
        let service: TorService = TorServiceParam {
            socks_port: Some(19051),
            data_dir: String::from("/tmp/torlib2"),
        }
        .into();
        assert_eq!(service.socks_port, 19051);
        assert_eq!(service.control_port.contains("127.0.0.1:"), true);
        assert_eq!(service._handle.is_some(), true);
        let mut control_conn = service.get_control_auth_conn(Some(handler)).await;
        let bootsraped = control_conn.wait_bootstrap().await.unwrap();
        assert_eq!(bootsraped, true);
        control_conn.take_ownership().await.unwrap();
        control_conn.shutdown();
        let _ = service._handle.unwrap().join();
    }

    #[test]
    #[serial(tor)]
    fn TorService_can_use_run_time_and_convert_to_OwnedTorservice() {
        let service: TorService = TorServiceParam {
            socks_port: Some(19054),
            data_dir: String::from("/tmp/sifir_rs_sdk/"),
        }
        .into();
        let client = service.get_client().unwrap();
        let mut owned_node = service.to_owned_node(None);
        (*RUNTIME).lock().unwrap().block_on(async {
            let resp = client
                .get("http://keybase5wmilwokqirssclfnsqrjdsi7jdir5wy7y7iu3tanwmtp6oid.onion")
                .send()
                .await
                .unwrap();
            assert_eq!(resp.status(), 200);
        });
        // take ctl and drop it
        owned_node.shutdown();
    }

    //#[test]
    //#[serial(tor)]
    //fn TorService_create_hidden_service() {
    //    let service: TorService<_> = TorServiceParam {
    //        port: 8000,
    //        socks_port: Some(19054),
    //        data_dir: String::from("/tmp/torlib3"),
    //    }
    //    .into();
    //    service.bootstrap_and_own(None);
    //    let mut ctl = service._ctl.borrow_mut().as_ref().unwrap();

    //    // FIXME need RunTime or move to ServiceAPI
    //    let service_key = ctl.create_hidden_service(TorHiddenServiceParam { port: 8080 });
    //    let client = service.get_client().unwrap();
    //    (*RUNTIME).lock().unwrap().block_on(async {
    //        let resp = client
    //            .get("http://keybase5wmilwokqirssclfnsqrjdsi7jdir5wy7y7iu3tanwmtp6oid.onion")
    //            .send()
    //            .await
    //            .unwrap();
    //        assert_eq!(resp.status(), 200);
    //    });

    //    // take ctl and drop it
    //    {
    //        let _ = service._ctl.into_inner().take();
    //    }
    //    let _ = service._handle.unwrap().join();
    //}
}
