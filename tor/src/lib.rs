pub mod tcp_stream;

// use crate::utils;
// use anyhow::{anyhow, Context, Result};
use crate::TorErrors::TorLibError;
use futures::{Future, TryStreamExt};
use lazy_static::*;
use libtor::{LogDestination, Tor, TorAddress, TorBool, TorFlag};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::convert::{TryFrom, TryInto};
use std::fmt;
use std::fs;
use std::io;
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::net::{TcpListener, ToSocketAddrs};
use std::pin::Pin;
use std::sync::Mutex;
use std::thread::JoinHandle;
use thiserror::Error;
use tokio::net::TcpStream;
use tokio::task::JoinError;
use tokio::time::{timeout, Duration};
use torut::control::{AsyncEvent, AuthenticatedConn, ConnError, UnauthenticatedConn};
use torut::onion::TorSecretKeyV3;

type F = Box<dyn Fn(AsyncEvent<'static>) -> Pin<Box<dyn Future<Output = Result<(), ConnError>>>>>;
type G = AuthenticatedConn<TcpStream, F>;
lazy_static! {
    pub static ref RUNTIME: Mutex<tokio::runtime::Runtime> = Mutex::new(
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
    pub bootstrap_timeout_ms: Option<u64>,
}

impl TorServiceParam {
    pub fn new(data_dir: &str, socks_port: u16, bootstap_timeout_ms: u64) -> TorServiceParam {
        TorServiceParam {
            data_dir: String::from(data_dir),
            socks_port: Some(socks_port),
            bootstrap_timeout_ms: Some(bootstap_timeout_ms),
        }
    }
}

pub struct TorService {
    socks_port: u16,
    control_port: String,
    bootstrap_timeout_ms: u64,
    _handle: Option<JoinHandle<Result<u8, libtor::Error>>>,
}

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
    fn wait_bootstrap(
        &mut self,
        timeout_ms: Option<u64>,
    ) -> Pin<Box<dyn Future<Output = Result<bool, TorErrors>> + '_>>;
    fn shutdown(self);
    fn get_status(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = Result<OwnedTorServiceBootstrapPhase, TorErrors>> + '_>>;
}

#[derive(Error, Debug)]
pub enum TorErrors {
    #[error("Control connection error: {:?}",.0)]
    ControlConnectionError(ConnError),
    #[error("Error with Tor daemon:")]
    TorLibError(#[from] libtor::Error),
    #[error("Error Bootstraping:")]
    BootStrapError(String),
    #[error("Error Bootstraping:")]
    IoError(#[from] io::Error),
    #[error("Error Threading:")]
    ThreadingError(#[from] JoinError),
}

/// Convert Torservice Param into an Unauthentication TorService:
/// Instantiates the Tor service on a seperate thread, however does not take ownership
/// nor await it's completion of the BootstrapPhase
// TODO make timeout a param, but how can we kill backgroun without having access ?
impl TryFrom<TorServiceParam> for TorService {
    type Error = TorErrors;
    fn try_from(param: TorServiceParam) -> Result<Self, Self::Error> {
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
            .flag(TorFlag::TruncateLogFile(TorBool::True))
            .flag(TorFlag::LogTo(
                libtor::LogLevel::Info,
                libtor::LogDestination::File(format!("{}sifir_sdk__tor_log.info", param.data_dir)),
            ))
            .flag(TorFlag::LogTo(
                libtor::LogLevel::Err,
                libtor::LogDestination::File(format!("{}sifir_sdk__tor_log.err", param.data_dir)),
            ));

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
                    if !t.contains("PORT=") {
                        return Err(TorErrors::BootStrapError(String::from("No port in config")));
                    };
                    let data: Vec<&str> = t.split("PORT=").collect();
                    control_port = data[1].into();
                    is_ready = true;
                }
                Err(e) => {
                    try_times += 1;
                    if try_times > 10 {
                        return Err(TorErrors::BootStrapError(String::from(
                            "ControlConnectionError",
                        )));
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(700));
        }

        Ok(TorService {
            socks_port,
            control_port,
            bootstrap_timeout_ms: param.bootstrap_timeout_ms.unwrap_or(45000),
            _handle: Some(handle),
        })
    }
}
/// Async handler injected into Torut to recieve Tor daemon async events
/// Right now does nothing but is needed for AuthenticatedConnection from Torut to function correctly
fn handler(_: AsyncEvent<'static>) -> Pin<Box<dyn Future<Output = Result<(), ConnError>> + '_>> {
    Box::pin(async move { Ok(()) })
}

impl TorService {
    pub fn new(param: TorServiceParam) -> Result<Self, TorErrors> {
        // FIXME store the param timeout here so we don't have to recall get owned with it
        param.try_into()
    }
    async fn get_control_auth_conn<F>(
        &self,
        handle: Option<F>,
    ) -> Result<AuthenticatedConn<TcpStream, F>, TorErrors> {
        let s = TcpStream::connect(self.control_port.trim()).await?;
        let mut utc = UnauthenticatedConn::new(s);
        // returns node info + cookie location ?
        let proto_info = utc
            .load_protocol_info()
            .await
            .map_err(TorErrors::ControlConnectionError)?;
        // loads cookie from loaded data and build auth info
        let auth = proto_info
            .make_auth_data()?
            .ok_or(TorErrors::BootStrapError(String::from(
                "Error making control auth data",
            )))?;
        utc.authenticate(&auth)
            .await
            .map_err(TorErrors::ControlConnectionError)?;
        // upgrade connection to authenticated
        let mut ac = utc.into_authenticated().await;
        if handle.is_some() {
            ac.set_async_event_handler(handle);
        }
        Ok(ac)
    }

    /// Converts TorService to OwnedTorService, consuming the TorService
    /// and returning an OwnedTorService which is fully bootstrapped and under our control
    /// (If we drop this object the Tor daemon will shut down)
    pub fn into_owned_node(self) -> Result<OwnedTorService, TorErrors> {
        (*RUNTIME).lock().unwrap().block_on(async {
            let mut ac = self
                .get_control_auth_conn(Some(Box::new(handler) as F))
                .await?;
            ac.wait_bootstrap(Some(self.bootstrap_timeout_ms)).await?;
            ac.take_ownership()
                .await
                .map_err(TorErrors::ControlConnectionError)?;
            Ok(OwnedTorService {
                socks_port: self.socks_port,
                control_port: self.control_port,
                _handle: self._handle,
                _ctl: RefCell::new(Some(ac)),
            })
        })
    }
}

impl TryFrom<TorServiceParam> for OwnedTorService {
    type Error = TorErrors;
    fn try_from(param: TorServiceParam) -> Result<Self, Self::Error> {
        let t: TorService = param.try_into()?;
        t.into_owned_node()
    }
}

/// Implementation when TorService has AuthenticatedConnection established
/// This is what the FFI and most external libs should be interacting with
impl OwnedTorService {
    pub fn new(param: TorServiceParam) -> Result<Self, TorErrors> {
        param.try_into()
    }
    // TODO check port is not already taken
    pub fn create_hidden_service(
        &mut self,
        param: TorHiddenServiceParam,
    ) -> Result<TorHiddenService, TorErrors> {
        (*RUNTIME).lock().unwrap().block_on(async {
            let mut _ctl = self._ctl.borrow_mut();
            let ctl = _ctl
                .as_mut()
                .ok_or(TorErrors::BootStrapError(String::from("Error mut lock")))?;

            let service_key = match param.secret_key {
                Some(key) => key.into(),
                _ => TorSecretKeyV3::generate(),
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
            .map_err(TorErrors::ControlConnectionError)?;

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
    pub fn get_status(&self) -> Result<OwnedTorServiceBootstrapPhase, TorErrors> {
        (*RUNTIME).lock().unwrap().block_on(async {
            let mut ctl = self._ctl.borrow_mut();
            let r = ctl
                .as_mut()
                .ok_or(TorErrors::BootStrapError("Unable to get mut".into()))?
                .get_status()
                .await?;
            Ok(r)
        })
    }
    /// take control conn and drop it.
    /// Closing the owned connection and causes tor daemon to shutdown
    /// Then waits on the Tor daemon thread to exit
    pub fn shutdown(&mut self) -> Result<(), TorErrors> {
        {
            let _ = self._ctl.borrow_mut().take();
        }
        let _ = self
            ._handle
            .take()
            .ok_or(TorErrors::BootStrapError(String::from(
                "Error shutdown take handle",
            )))?
            .join()
            .map_err(|e| TorErrors::BootStrapError(String::from("Error joining on shutdown")))?;
        Ok(())
    }
}
/// High level API for Torut used internally by TorService to expose
/// note control functions to FFI and user
impl<F, H> TorControlApi for AuthenticatedConn<TcpStream, H>
where
    H: Fn(AsyncEvent<'static>) -> F,
    F: Future<Output = Result<(), ConnError>>,
{
    fn wait_bootstrap(
        &mut self,
        timeout_ms: Option<u64>,
    ) -> Pin<Box<dyn Future<Output = Result<bool, TorErrors>> + '_>> {
        // Wait for boostrap to be done
        let future = async move {
            timeout(
                Duration::from_millis(timeout_ms.unwrap_or(15000)),
                async move {
                    let mut input = String::new();
                    while !input.trim().contains("PROGRESS=100 TAG=done") {
                        input = self
                            .get_info("status/bootstrap-phase")
                            .await
                            .map_err(TorErrors::ControlConnectionError)?;
                        std::thread::sleep(std::time::Duration::from_millis(300));
                    }
                    Ok(true)
                },
            )
            .await
            .map_err(|e| TorErrors::BootStrapError(String::from("Timeout waiting for boostrap")))?
        };
        Box::pin(future)
    }
    fn get_status(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = Result<OwnedTorServiceBootstrapPhase, TorErrors>> + '_>> {
        // Wait for boostrap to be done
        Box::pin(async move {
            let input = self
                .get_info("status/bootstrap-phase")
                .await
                .map_err(TorErrors::ControlConnectionError)?;
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
    use std::convert::TryInto;
    use std::io::{Read, Write};
    use std::net::{TcpListener, ToSocketAddrs};

    #[test]
    #[serial(tor)]
    fn get_from_param_and_await_boostrap_using_TorControlApi() {
        (*RUNTIME).lock().unwrap().block_on(async {
            let service: TorService = TorServiceParam {
                socks_port: Some(19051),
                data_dir: String::from("/tmp/torlib2"),
                bootstrap_timeout_ms: Some(45000),
            }
            .try_into()
            .unwrap();
            assert_eq!(service.socks_port, 19051);
            assert_eq!(service.control_port.contains("127.0.0.1:"), true);
            assert_eq!(service._handle.is_some(), true);
            let mut control_conn = service.get_control_auth_conn(Some(handler)).await.unwrap();
            let bootsraped = control_conn.wait_bootstrap(Some(20000)).await.unwrap();
            assert_eq!(bootsraped, true);
            control_conn.take_ownership().await.unwrap();
            control_conn.shutdown();
            let _ = service._handle.unwrap().join();
        });
    }

    #[test]
    #[serial(tor)]
    fn bootstrap_timeout() {
        (*RUNTIME).lock().unwrap().block_on(async {
            let service: TorService = TorServiceParam {
                socks_port: Some(19051),
                data_dir: String::from("/tmp/torlib2"),
                bootstrap_timeout_ms: Some(1000),
            }
            .try_into()
            .unwrap();
            assert_eq!(service.socks_port, 19051);
            assert_eq!(service.control_port.contains("127.0.0.1:"), true);
            assert_eq!(service._handle.is_some(), true);
            let mut control_conn = service.get_control_auth_conn(Some(handler)).await.unwrap();
            let bootsraped = control_conn.wait_bootstrap(Some(500)).await;
            assert_eq!(bootsraped.is_err(), true);
        });
    }

    #[test]
    #[serial(tor)]
    fn to_owned() {
        let service: TorService = TorServiceParam {
            socks_port: Some(19054),
            data_dir: String::from("/tmp/sifir_rs_sdk/"),
            bootstrap_timeout_ms: Some(45000),
        }
        .try_into()
        .unwrap();
        let client = utils::get_proxied_client(service.socks_port).unwrap();

        let mut owned_node = service.into_owned_node().unwrap();
        (*RUNTIME).lock().unwrap().block_on(async {
            let resp = client
                .get("http://keybase5wmilwokqirssclfnsqrjdsi7jdir5wy7y7iu3tanwmtp6oid.onion")
                .send()
                .await
                .unwrap();
            assert_eq!(resp.status(), 200);
        });
        // take ctl and drop it
        owned_node.shutdown().unwrap();
    }

    #[test]
    #[serial(tor)]
    fn to_owned_with_timeout() {
        let service: TorService = TorServiceParam {
            socks_port: Some(19054),
            data_dir: String::from("/tmp/sifir_rs_sdk/"),
            bootstrap_timeout_ms: Some(30000),
        }
        .try_into()
        .unwrap();
        assert_eq!(service.into_owned_node().is_err(), true);
    }

    #[test]
    #[serial(tor)]
    fn get_status_of_OwnedTorService() {
        let service: TorService = TorServiceParam {
            socks_port: Some(19054),
            data_dir: String::from("/tmp/sifir_rs_sdk/"),
            bootstrap_timeout_ms: Some(45000),
        }
        .try_into()
        .unwrap();
        let mut owned_node = service.into_owned_node().unwrap();
        let status = owned_node.get_status().unwrap();
        assert!(matches!(status, OwnedTorServiceBootstrapPhase::Done));
        owned_node.shutdown().unwrap();
    }
    #[test]
    #[serial(tor)]
    fn TorService_create_hidden_service() {
        let service: TorService = TorServiceParam {
            socks_port: Some(19054),
            data_dir: String::from("/tmp/sifir_rs_sdk/"),
            bootstrap_timeout_ms: Some(45000),
        }
        .try_into()
        .unwrap();
        let client = utils::get_proxied_client(service.socks_port).unwrap();
        let mut owned_node = service.into_owned_node().unwrap();
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
        owned_node.shutdown().unwrap();
    }
}
