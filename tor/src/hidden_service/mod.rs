use crate::TorErrors;
use crate::RUNTIME;
use logger::log::*;
use socks::Socks5Stream;
use std::borrow::{Borrow, BorrowMut};
use std::io::BufRead;
use std::io::{Read, Write};
use std::net::Shutdown;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use tokio::io::{
    AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWriteExt, BufReader, ReadBuf,
};
use tokio::net::TcpStream;
use tokio::net::{TcpListener, ToSocketAddrs};
// use tokio::stream::StreamExt;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{timeout, Duration};
use tokio_compat_02::FutureExt;
use torut::control::TorErrorKind;

type HiddenServiceDataHandler = Box<dyn DataObserver + Send + Sync + 'static>;

pub struct HiddenServiceHandler {
    port: u32,
    data_handler: Arc<RwLock<Option<HiddenServiceDataHandler>>>,
}

pub trait DataObserver {
    fn on_data(&self, data: String);
    fn on_error(&self, data: String);
}

impl HiddenServiceHandler {
    fn new(port: u32) -> Result<Self, TorErrors> {
        // FIXME listening port ?
        // shut down function ?
        // b64 data ?
        // 500 status code on errors ?
        Ok(HiddenServiceHandler {
            port,
            data_handler: Arc::new(RwLock::new(None)),
        })
    }
    pub fn set_data_handler<F>(&self, callback: F) -> Result<(), TorErrors>
    where
        F: DataObserver + Send + Sync + 'static,
    {
        (*RUNTIME).lock().unwrap().block_on(async move {
            let data_clone = self.data_handler.clone();
            let mut data_write = data_clone.write().await;
            *data_write = Some(Box::new(callback));
        });
        Ok(())
    }

    pub fn read_async(&mut self) -> Result<(), TorErrors> {
        let cb_clone = self.data_handler.clone();
        let port = self.port;
        (*RUNTIME).lock().unwrap().spawn(async move {
            let mut listener = TcpListener::bind(format!("127.0.0.1:{},", port))
                .await
                .unwrap();
            match listener.accept().await {
                Ok((mut stream, addr)) => {
                    info!("new client: {:?}", addr);
                    let mut string_buf = String::new();
                    let read_result = stream.read_to_string(&mut string_buf).await;
                    let cb_option = cb_clone.write().await;

                    if let Some(ref mut cb) = cb_option.as_ref() {
                        match read_result {
                            Ok(size) => {
                                if size == 0 {
                                    cb.on_error(String::from("EOF"));
                                    warn!("EOF")
                                    //println!("Rust:Tor:TcpStream.ondata: EOF detected for read stream, shutting down streams..");
                                    //// if we error out on shutdown not a biggie, just log it
                                    //if let Err(e) = stream.shutdown(Shutdown::Write) {
                                    //    cb.on_error(format!("Rust:Tor:TcpStream.onData: EOF Shutdown Write: {:?}", e));
                                    //}
                                    //if let Err(e) = tcp_stream.shutdown(Shutdown::Read) {
                                    //    cb.on_error(format!("Rust:Tor:TcpStream.onData: EOF Shutdown Read: {:?}", e));
                                    //}
                                } else {
                                    info!("Sending {} bytes", size);
                                    cb.on_data(string_buf)
                                }
                            }
                            Err(e) => cb.on_error(e.to_string()),
                        }
                    }
                }
                Err(e) => {
                    error!("couldn't get client: {:?}", e)
                }
            }
        });
        Ok(())
    }

    pub fn shutdown(&mut self) -> Result<(), TorErrors> {
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{OwnedTorService, TorHiddenServiceParam, TorService, TorServiceParam};
    use serial_test::serial;
    use std::borrow::{Borrow, BorrowMut};
    use std::convert::TryInto;
    use std::ops::Deref;
    use std::sync::{Arc, Mutex};

    #[test]
    fn hidden_service_handler() {
        let socks_port = 19054;
        let mut owned_node: OwnedTorService = TorServiceParam {
            socks_port: Some(socks_port),
            data_dir: String::from("/tmp/sifir_rs_sdk"),
            bootstrap_timeout_ms: Some(45000),
        }
        .try_into()
        .unwrap();
        let service_key = owned_node
            .create_hidden_service(TorHiddenServiceParam {
                to_port: 20000,
                hs_port: 20011,
                secret_key: None,
            })
            .unwrap();
        assert!(service_key.onion_url.to_string().contains(".onion"));

        struct Observer {
            pub count: Arc<Mutex<u16>>,
        }
        impl DataObserver for Observer {
            fn on_data(&self, data: String) {
                let mut count = self.count.lock().unwrap();
                *count += 1;
                println!("Got data call number {} with {} ", count, data);
                assert_eq!(data.contains("rpc"), true);
            }
            fn on_error(&self, data: String) {
                if data != "EOF" {
                    panic!("Got error!: {}", data);
                }
            }
        }
        let count = Arc::new(Mutex::new(0));
        let obv = Observer {
            count: count.clone(),
        };

        let mut listner = HiddenServiceHandler::new(20000).unwrap();
        let _ = listner.set_data_handler(obv).unwrap();

        (*RUNTIME).lock().unwrap().block_on(
            async {
                let client = utils::get_proxied_client(socks_port).unwrap();
                let mut onion_url =
                    utils::reqwest::Url::parse(&format!("http://{}", service_key.onion_url))
                        .unwrap();
                let _ = onion_url.set_port(Some(20011 as u16));

                let resp = client.get(onion_url).send().await.unwrap();
                assert_eq!(resp.status(), 200);
            }
            .compat(),
        );
        owned_node.shutdown().unwrap();
    }
}
