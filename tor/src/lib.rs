use anyhow::Result;
use libtor::{HiddenServiceVersion, Tor, TorAddress, TorFlag};
use serde::{Deserialize, Serialize};
#[cfg(test)]
use serial_test::serial;
use std::fs;
use std::pin::Pin;
use std::thread::JoinHandle;
use tokio::macros::support::Future;
use tokio::net::TcpStream;
use torut::control::{AsyncEvent, AuthenticatedConn, ConnError, UnauthenticatedConn};
use lazy_static::*;
use std::sync::{Mutex,Arc};
use std::cell::RefCell;

type E = Pin<Box<dyn Future<Output = Result<bool, ()>>>>;
type F = Box<dyn Fn(AsyncEvent<'static>) -> E>;
type G = AuthenticatedConn<TcpStream,F>;

lazy_static! {
     //runtime with threaded pool
     static ref RUNTIME: Mutex<tokio::runtime::Runtime> = Mutex::new(tokio::runtime::Builder::new()
         .threaded_scheduler()
         .enable_all()
         .build()
         .unwrap());
     static ref CTL_CONN: Arc<Mutex<Option<G>>> = Arc::new(Mutex::new(None));
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TorServiceParam {
    port: u16,
    socks_port: Option<u16>,
    data_dir: String,
}

pub struct TorService {
    socks_port: u16,
    control_port: String,
    _handle: Option<JoinHandle<Result<u8, libtor::Error>>>,
    // _ctl: RefCell<Option<AuthenticatedConn<TcpStream,F>>>
}

trait TorServiceStatus {
    // async fns in traits are a shitshow
    fn wait_bootstrap( &mut self) -> Pin<Box<dyn Future<Output = Result<bool, ()>> + '_>>;
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
                    if try_times > 4 {
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
            // _ctl: RefCell::new(None),
        }
    }
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
	pub fn take_control(&self){
		(*RUNTIME).lock().unwrap().block_on(async {
				let mut ac = self.get_control_auth_conn(Some(|event| async move 
				{
                	println!("{:#?}", event);
                	Ok(())
            	})).await;
                ac.wait_bootstrap().await.unwrap();
                let mut conn = (*CTL_CONN).lock().unwrap();
                 *conn = Some(ac);
                // STORE THIS!
                // *self._ctl.borrow_mut() = Some(ac);dd
		});
    }
    async fn get_control_auth_conn<H,F>(&self, handle: Option<H>) -> AuthenticatedConn<TcpStream, H> 
where
    H: Fn(AsyncEvent<'static>) -> F,
    F: Future<Output = Result<(), ConnError>>,
{
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
}

impl<F, H> TorServiceStatus for AuthenticatedConn<TcpStream, H>
where
    H: Fn(AsyncEvent<'static>) -> F,
    F: Future<Output = Result<(), ConnError>>,
{
    fn wait_bootstrap(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = Result<bool, ()>> + '_>> {
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
    #[serial(tor)]
    async fn get_from_param_and_await_boostrap() {
        let service: TorService = TorServiceParam {
            port: 8000,
            socks_port: Some(19051),
            data_dir: String::from("/tmp/torlib2"),
        }
        .into();
        assert_eq!(service.socks_port, 19051);
        assert_eq!(service.control_port.contains("127.0.0.1:"), true);
        assert_eq!(service._handle.is_some(), true);
        let mut control_conn = service
            .get_control_auth_conn(Some(|_: AsyncEvent<'static>| async move { Ok(()) }))
            .await;
        let _ = control_conn.wait_bootstrap().await;
        control_conn.take_ownership().await;
        control_conn.shutdown();
        let _ = service._handle.unwrap().join();
    }

    #[tokio::test]
    #[serial(tor)]
    async fn should_get_onion_url() {
        let service: TorService = TorServiceParam {
            port: 8000,
            socks_port: Some(19054),
            data_dir: String::from("/tmp/torlib3"),
        }
        .into();
        let mut control_conn = service.take_control();
        let client = service.get_client().unwrap();
        let resp = client
            .get("http://keybase5wmilwokqirssclfnsqrjdsi7jdir5wy7y7iu3tanwmtp6oid.onion")
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
        control_conn.shutdown();
        let _ = service._handle.unwrap().join();
    }
}
