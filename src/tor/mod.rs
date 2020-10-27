use libtor::{HiddenServiceVersion, Tor, TorAddress, TorFlag};
use std::thread::JoinHandle;
pub struct TorService {
    port: u16,
    socks_port: u16,
    data_dir: String,
    _handle: Option<JoinHandle<Result<u8, libtor::Error>>>,
}

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

impl TorService {
    fn new() -> Self {
        TorService {
            port: 8000,
            socks_port: 19050,
            data_dir: String::from("/tmp/tor-rust"),
            _handle: None,
        }
    }
    pub fn start_service(&mut self) -> Result<(), libtor::Error> {
        let data_dir = &self.data_dir;
        let mut service = Tor::new();
        service
            .flag(TorFlag::DataDirectory(data_dir.into()))
            .flag(TorFlag::SocksPort(self.socks_port))
            .flag(TorFlag::TestSocks(libtor::TorBool::True));
        //.flag(TorFlag::HiddenServiceDir(data_dir.into()))
        //.flag(TorFlag::HiddenServiceVersion(HiddenServiceVersion::V3))
        //.flag(TorFlag::HiddenServicePort(
        //    TorAddress::Port(self.port),
        //    None.into(),
        //));
        let handle = service.start_background();
        self._handle = Some(handle);
        println!("Sleeping..");
        let ten_millis = std::time::Duration::from_secs(7);
        std::thread::sleep(ten_millis);
        Ok(())
    }
    pub fn stop_service(self) {
        match self._handle {
            Some(x) => {
                x.join().expect("Error waiting for service thread");
            }
            None => {
                println!("No service detected");
            }
        }
    }

    fn get_client(&self) -> Result<reqwest::Client, reqwest::Error> {
        let proxy = reqwest::Proxy::all(
            reqwest::Url::parse(format!("socks5h://127.0.0.1:{}", self.socks_port).as_str())
                .unwrap(),
        )
        .unwrap();
        reqwest::Client::builder().proxy(proxy).build()
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn should_bootstrap_a_socksh_proxy() {
        println!("Hello, world!");
        let mut tor = TorService::new();
        tor.start_service().unwrap();
        println!("Tor servicce started");
        let client = tor.get_client().unwrap();
        let resp = client
            .get("http://keybase5wmilwokqirssclfnsqrjdsi7jdir5wy7y7iu3tanwmtp6oid.onion")
            .send()
            .await.unwrap();
        println!("Call copmlete");
        println!("{:#?}", resp);
        // tor.stop_service();
        assert_eq!(resp.status(),200);
    }
}
