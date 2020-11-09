use anyhow::Result;
use serde::{Deserialize, Serialize};
use serial_test::serial;
// use std::cell::RefCell;
// use std::future::Future;
use libtor::{HiddenServiceVersion, Tor, TorAddress, TorFlag};
use std::fs;
use std::marker::Unpin;
use std::pin::Pin;
use std::thread::JoinHandle;
use tokio::io::{AsyncRead, AsyncWrite};
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

trait TorServiceStatus {
    // async fns in traits are a shitshow
    fn wait_bootstrap_and_own(
        &mut self,
        take_ownership: Option<bool>,
    ) -> Pin<Box<dyn Future<Output = Result<bool, ()>> + '_>>;
    fn shutdown(self);
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

        let mut isReady = false;
        //while !isReady {
        //    let contents = fs::read_to_string(format!("{}/state", param.data_dir.clone()));
        //    match contents {
        //        Ok(T) => {
        //            if T.contains("CircuitBuildTimeBin") {
        //                isReady = true;
        //                continue;
        //            }
        //        }
        //        Err(e) => {

        //            // Try a couple of more times before stopping
        //        }
        //    }
        //    std::thread::sleep(std::time::Duration::from_millis(300));
        //}
        // FIXME go get the config file from the dir once ConfigAuto is fixed
        // save it control port etc info

        std::thread::sleep(std::time::Duration::from_millis(1000));
        TorService {
            socks_port,
            control_port,
            _handle: Some(handle),
        }
    }
}
impl TorService {
    pub fn get_client(&self) -> Result<reqwest::Client, reqwest::Error> {
        let proxy = reqwest::Proxy::all(
            reqwest::Url::parse(format!("socks5h://127.0.0.1:{}", self.socks_port).as_str())
                .unwrap(),
        )
        .unwrap();
        reqwest::Client::builder().proxy(proxy).build()
    }
    async fn get_control_auth_conn<T>(&self, handle: Option<T>) -> AuthenticatedConn<TcpStream, T> {
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
    // FROM
    //https://users.rust-lang.org/t/solved-is-it-possible-to-run-async-code-in-a-trait-method-with-stdfuture-async-await/24874/2
    fn wait_bootstrap_and_own(
        &mut self,
        take_ownership: Option<bool>,
    ) -> Pin<Box<dyn Future<Output = Result<bool, ()>> + '_>> {
        // Wait for boostrap to be done
        Box::pin(async move {
            let mut input = String::new();
            while !input.trim().contains("PROGRESS=100 TAG=done") {
                input = self.get_info("status/bootstrap-phase").await.unwrap();
                std::thread::sleep(std::time::Duration::from_millis(300));
            }
            // Default to takeownership unless expcility says no
            if take_ownership.unwrap_or(true) {
                self.take_ownership().await;
            }
            Ok(true)
        })
    }
    fn shutdown(self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[serial]
    async fn get_from_param_and_await_boostrap() {
        let service: TorService = TorServiceParam {
            port: 8000,
            socks_port: Some(19051),
            data_dir: String::from("/tmp/torlib2"),
        }
        .into();
        assert_eq!(service.socks_port, 19051);
        assert_eq!(service.control_port, 19052);
        assert_eq!(service._handle.is_some(), true);
        let mut control_conn = service
            .get_control_auth_conn(Some(|event: AsyncEvent<'static>| async move { Ok(()) }))
            .await;
        let _ = control_conn.wait_bootstrap_and_own(Some(true)).await;
        control_conn.shutdown();
        let _ = service._handle.unwrap().join();
    }

    #[tokio::test]
    #[serial]
    async fn should_get_onion_url() {
        let service: TorService = TorServiceParam {
            port: 8000,
            socks_port: Some(19051),
            data_dir: String::from("/tmp/torlib2"),
        }
        .into();
        let mut control_conn = service
            .get_control_auth_conn(Some(|event: AsyncEvent<'static>| async move {
                println!("{:#?}", event);
                Ok(())
            }))
            .await;
        let _ = control_conn.wait_bootstrap_and_own(Some(true)).await;

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
    // Tell bootstraping is done
    // let info = service.get_bootstarp_phase().await;
    // assert_eq!(info.contains("PROGRESS=100 TAG=done"), true)
    //}
}
