// use crate::utils;
use anyhow::Result;
use futures::Future;
use lazy_static::*;
use libtor::{LogDestination, Tor, TorAddress, TorBool, TorFlag};
use serde::{Deserialize, Serialize};
use socks::Socks5Stream;
use socks::{Socks5Datagram, ToTargetAddr};
use std::cell::RefCell;
use std::fs;
use std::io::BufRead;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::net::{TcpListener, ToSocketAddrs};
use std::pin::Pin;
use std::sync::Mutex;
use std::thread::JoinHandle;
use tokio::net::TcpStream;
use torut::control::{AsyncEvent, AuthenticatedConn, ConnError, UnauthenticatedConn};
use torut::onion::TorSecretKeyV3;
/// TODO implement this for hidden service
pub enum CallBackResult {
    Success(String),
    Error(String),
}
pub trait CallBack {
    fn on_state_changed(&self, result: CallBackResult);
}

type F = Box<dyn Fn(AsyncEvent<'static>) -> Pin<Box<dyn Future<Output = Result<(), ConnError>>>>>;
type G = AuthenticatedConn<TcpStream, F>;
lazy_static! {
    static ref RUNTIME: Mutex<tokio::runtime::Runtime> = Mutex::new(
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
    );
}

#[repr(C)]
#[derive(Serialize, Deserialize, Debug)]
pub struct TorServiceParam {
    pub socks_port: Option<u16>,
    pub data_dir: String,
}

impl TorServiceParam {
    /// A constructor for TorServiceParam to make it easier to be called from
    /// an FFI
    pub fn new(data_dir: &str, socks_port: u16) -> TorServiceParam {
        TorServiceParam {
            data_dir: String::from(data_dir),
            socks_port: Some(socks_port),
        }
    }
}

pub struct TorService {
    socks_port: u16,
    control_port: String,
    _handle: Option<JoinHandle<Result<u8, libtor::Error>>>,
}

#[repr(C)]
pub struct OwnedTorService {
    pub socks_port: u16,
    pub control_port: String,
    _handle: Option<JoinHandle<Result<u8, libtor::Error>>>,
    _ctl: RefCell<Option<G>>,
}

#[repr(C)]
pub struct TorHiddenServiceParam {
    pub to_port: u16,
    pub hs_port: u16,
    pub secret_key: Option<[u8; 64]>,
}

pub struct TorHiddenService {
    pub onion_url: TorAddress,
    pub secret_key: [u8; 64],
}

#[repr(C)]
pub struct MsgOverTcp {
    pub target: String,
    pub msg: String,
}
//trait TorSocksProxy {
//    fn get_socks_port(&self) -> u16;
//}

/// The Phases of a Boostraping node
/// From https://github.com/torproject/torspec/blob/master/proposals/137-bootstrap-phases.txt
#[repr(C)]
#[derive(Serialize, Deserialize, Debug)]
/// String describing the current bootstarp phase of the node
pub struct BootstrapPhase(String);

#[repr(C)]
#[derive(Serialize, Deserialize, Debug)]
/// Describes the BootstrapPhase the Tor daemon is in.
pub enum OwnedTorServiceBootstrapPhase {
    // Daemon is done Boostraping and is ready to use
    Done,
    // Still bootstraping or error
    Other(BootstrapPhase),
}
/// High level API for Torut's AuthenticatedConnection used internally by TorService to expose
/// note control functions to FFI and user
trait TorControlApi {
    // async fns in traits are a shit show
    fn wait_bootstrap(&mut self) -> Pin<Box<dyn Future<Output = Result<bool, ()>> + '_>>;
    fn shutdown(self);
    fn get_status(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = Result<OwnedTorServiceBootstrapPhase, ()>> + '_>>;
}

/// Convert Torservice Param into an Unauthentication TorService:
/// Instantiates the Tor service on a seperate thread, however does not take ownership
/// nor await it's completion of the BootstrapPhase
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
            .flag(TorFlag::ControlPortFileGroupReadable(libtor::TorBool::True))
            .flag(TorFlag::TruncateLogFile(TorBool::True));
        // FIXME bug in flag or permissions ?
        //.flag(TorFlag::LogTo(
        //    libtor::LogLevel::Info,
        //    libtor::LogDestination::File(format!("{}tor_log.info", param.data_dir)),
        //))
        //.flag(TorFlag::LogTo(
        //    libtor::LogLevel::Err,
        //    libtor::LogDestination::File(format!("{}tor_log.err", param.data_dir)),
        //));

        let handle = service.start_background();

        let mut is_ready = false;
        let mut control_port = String::new();
        let mut try_times = 0;
        // TODO We wait for Tor to write the new config file otherwise we risk reading the old config and port.
        // Anything less than a second and iOS errors out
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
                    if try_times > 10 {
                        panic!("Could load Tor control config file");
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(700));
        }

        TorService {
            socks_port,
            control_port,
            _handle: Some(handle),
        }
    }
}
/// Async handler injected into Torut to recieve Tor daemon async events
/// Right now does nothing but is needed for AuthenticatedConnection from Torut to function correctly
fn handler(_: AsyncEvent<'static>) -> Pin<Box<dyn Future<Output = Result<(), ConnError>> + '_>> {
    Box::pin(async move { Ok(()) })
}

impl TorService {
    pub fn new(param: TorServiceParam) -> Self {
        param.into()
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

    /// Converts TorService to OwnedTorService, consuming the TorService
    /// and returning an OwnedTorService which is fully bootstrapped and under our control
    /// (If we drop this object the Tor daemon will shut down)
    pub fn into_owned_node(self, callback: Option<Box<dyn CallBack>>) -> OwnedTorService {
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
        t.into_owned_node(None)
    }
}

/// Implementation when TorService has AuthenticatedConnection established
/// This is what the FFI and most external libs should be interacting with
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
            let service_key = {
                if param.secret_key.is_some() {
                    param.secret_key.unwrap().into()
                } else {
                    TorSecretKeyV3::generate()
                }
            };
            ctl.add_onion_v3(
                &service_key,
                false,
                false,
                false,
                None,
                &mut [(
                    param.hs_port,
                    SocketAddr::new(IpAddr::from(Ipv4Addr::new(127, 0, 0, 1)), param.to_port),
                )]
                .iter(),
            )
            .await
            .unwrap();
            let onion_url = TorAddress::AddressPort(
                service_key.public().get_onion_address().to_string(),
                param.hs_port,
            );
            let secret_key = service_key.as_bytes();
            Ok(TorHiddenService {
                onion_url,
                secret_key,
            })
        })
    }

    /// Get the status of the Tor daemon we own
    /// OwnedTorServiceBootstrapPhase will either be Done or Other(String) containing the stage of
    /// the boostrap the node is a
    pub fn get_status(&self) -> Result<OwnedTorServiceBootstrapPhase> {
        (*RUNTIME).lock().unwrap().block_on(async {
            let mut ctl = self._ctl.borrow_mut();
            Ok(ctl.as_mut().unwrap().get_status().await.unwrap())
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
    /// Send a message through Socks5 proxy over a Raw TCP socket
    /// Connections are not persistant
    /// Note: param.msg is converted to .as_bytes here. Idea is most of this is coming across FFI so b64 > binary for cross
    /// barrier compatiblity
    /// If a callback is supplied will call the callback when the result is ready
    /// otherwise will block until it recieves a reply and return it.
    pub fn msg_over_tcp<F>(&self, param: MsgOverTcp,callback:Option<F>)
        -> anyhow::Result<Option<String>> where F:FnOnce(String) + Send + 'static,
    {
        let proxy = format!("127.0.0.1:{}", self.socks_port);
        let mut conn = Socks5Stream::connect(proxy.as_str(), param.target.as_str())
            .unwrap()
            .into_inner();
        // Setup lnser before sending
        let mut reader = BufReader::new(conn.try_clone()?);
        let callback_is_none = callback.is_none();
        let lsnr_handle = (*RUNTIME).lock().unwrap().spawn(async move {
            let mut string_buf = String::new();
            let _ = reader.read_line(&mut string_buf).unwrap();
            match callback{
                Some(cb)=>{
                    cb(string_buf);
                    None
                },
                _=>{
                   Some(string_buf)
                }
            }
        });
        conn.write_all(param.msg.as_bytes()).unwrap();

        if callback_is_none {
            let result = (*RUNTIME)
                .lock()
                .unwrap()
                .block_on(async { lsnr_handle.await.unwrap() });
            Ok(result)
        } else {
            Ok(None)
        }

    }
}

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
                std::thread::sleep(std::time::Duration::from_millis(300));
            }
            Ok(true)
        })
    }
    fn get_status(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = Result<OwnedTorServiceBootstrapPhase, ()>> + '_>> {
        // Wait for boostrap to be done
        Box::pin(async move {
            let input = self.get_info("status/bootstrap-phase").await.unwrap();
            if input.trim().contains("TAG=done") {
                Ok(OwnedTorServiceBootstrapPhase::Done)
            } else {
                Ok(OwnedTorServiceBootstrapPhase::Other(BootstrapPhase(
                    input.trim().into(),
                )))
            }
        })
    }
    // dropping the control connection after having taken ownership of the node will cause the node
    // to shutdown
    fn shutdown(self) {}
}
#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use socks::{Socks5Datagram, ToTargetAddr};
    use std::borrow::Borrow;
    use std::io::{Read, Write};
    use std::net::{TcpListener, ToSocketAddrs};

    //#[tokio::test]
    //#[serial(tor)]
    //async fn get_from_param_and_await_boostrap_using_TorControlApi() {
    //    let service: TorService = TorServiceParam {
    //        socks_port: Some(19051),
    //        data_dir: String::from("/tmp/torlib2"),
    //    }
    //    .into();
    //    assert_eq!(service.socks_port, 19051);
    //    assert_eq!(service.control_port.contains("127.0.0.1:"), true);
    //    assert_eq!(service._handle.is_some(), true);
    //    let mut control_conn = service.get_control_auth_conn(Some(handler)).await;
    //    let bootsraped = control_conn.wait_bootstrap().await.unwrap();
    //    assert_eq!(bootsraped, true);
    //    control_conn.take_ownership().await.unwrap();
    //    control_conn.shutdown();
    //    let _ = service._handle.unwrap().join();
    //}

    #[test]
    #[serial(tor)]
    fn TorService_can_use_run_time_and_convert_to_OwnedTorservice() {
        let service: TorService = TorServiceParam {
            socks_port: Some(19054),
            data_dir: String::from("/tmp/sifir_rs_sdk/"),
        }
        .into();
        let client = utils::get_proxied_client(service.socks_port).unwrap();

        let mut owned_node = service.into_owned_node(None);
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

    #[test]
    #[serial(tor)]
    fn get_status_of_OwnedTorService() {
        let service: TorService = TorServiceParam {
            socks_port: Some(19054),
            data_dir: String::from("/tmp/sifir_rs_sdk/"),
        }
        .into();
        let mut owned_node = service.into_owned_node(None);
        let status = owned_node.get_status().unwrap();
        assert!(matches!(status, OwnedTorServiceBootstrapPhase::Done));
        owned_node.shutdown();
    }
    #[test]
    #[serial(tor)]
    fn TorService_create_hidden_service() {
        let service: TorService = TorServiceParam {
            socks_port: Some(19054),
            data_dir: String::from("/tmp/sifir_rs_sdk/"),
        }
        .into();
        let client = utils::get_proxied_client(service.socks_port).unwrap();
        let mut owned_node = service.into_owned_node(None);
        let service_key = owned_node
            .create_hidden_service(TorHiddenServiceParam {
                to_port: 20000,
                hs_port: 20011,
                secret_key: None,
            })
            .unwrap();
        assert!(service_key.onion_url.to_string().contains(".onion"));

        // Spawn a lsner to our request and respond with 200
        let handle = (*RUNTIME).lock().unwrap().spawn(async {
            let listener = TcpListener::bind("127.0.0.1:20000").unwrap();
            for stream in listener.incoming() {
                let mut stream = stream.unwrap();
                let response = "HTTP/1.1 200 OK\r\n\r\n";
                stream.write_all(response.as_bytes()).unwrap();
                stream.flush().unwrap();
            }
        });

        let mut onion_url =
            utils::reqwest::Url::parse(&format!("http://{}", service_key.onion_url)).unwrap();
        let _ = onion_url.set_port(Some(20011 as u16));

        (*RUNTIME).lock().unwrap().block_on(async {
            let resp = client.get(onion_url).send().await.unwrap();
            assert_eq!(resp.status(), 200);
        });
        owned_node.shutdown();
    }
    #[test]
    #[serial(tor)]
    fn TorService_can_connect_raw_stream() {
        let service: TorService = TorServiceParam {
            socks_port: Some(19054),
            data_dir: String::from("/tmp/sifir_rs_sdk/"),
        }
        .into();
        let mut owned_node = service.into_owned_node(None);

        let target = "udfpzbte2hommnvag5f3qlouqkhvp3xybhlus2yvfeqdwlhjroe4bbyd.onion:60001";
        let msg = "{ \"id\": 1, \"method\": \"blockchain.scripthash.get_balance\", \"params\": [\"716decbe1660861c3d93906cb1d98ee68b154fd4d23aed9783859c1271b52a9c\"] }\n";

        // Test callback interface
         owned_node
            .msg_over_tcp(MsgOverTcp {
                target: target.into(),
                msg: msg.into(),
            },Some(|reply:String| {
                assert_eq!(reply.contains("rpc"), true);
            })).unwrap();

        // Test blocking interface
        let reply : Result<Option<String>> = owned_node
            .msg_over_tcp(MsgOverTcp { target: target.into(), msg: msg.into() },None::<fn(String)>);
        assert_eq!(reply.unwrap().unwrap().contains("rpc"),true);
        owned_node.shutdown();
    }
}
