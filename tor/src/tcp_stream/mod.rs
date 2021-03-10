use crate::TorErrors;
use crate::RUNTIME;
use socks::Socks5Stream;
use std::io::BufRead;
use std::io::BufReader;
use std::io::{Read, Write};
use std::net::Shutdown;
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

pub struct TcpSocksStream {
    target: String,
    socks_proxy: String,
    stream: Socks5Stream,
}

pub trait DataObserver {
    fn on_data(&self, data: String);
    fn on_error(&self, data: String);
}

impl TcpSocksStream {
    /// Blocks indefinitely until connection established
    pub fn new(target: String, socks_proxy: String) -> Result<Self, TorErrors> {
        let socks_stream = Socks5Stream::connect(socks_proxy.as_str(), target.as_str())?;
        Ok(TcpSocksStream {
            target,
            socks_proxy,
            stream: socks_stream,
        })
    }
    /// New (connect) but with a timeout
    /// Blocks till connection established or timeout (in MS) expires
    pub fn new_timeout(
        target: String,
        socks_proxy: String,
        timeout_ms: u64,
    ) -> Result<Self, TorErrors> {
        let socks_future = (*RUNTIME)
            .lock()
            .unwrap()
            .spawn(async move { TcpSocksStream::new(target, socks_proxy) });

        (*RUNTIME)
            .lock()
            .unwrap()
            .block_on(async move { timeout(Duration::from_millis(timeout_ms), socks_future).await })
            .map_err(|_| TorErrors::BootStrapError(String::from("Tcp connection timedout")))?
            .map_err(TorErrors::ThreadingError)?
    }
    /// Spawns a new lsnr on the tcp stream that will call the passed callback with bufsize bytes of
    /// base64 encoded string of data as it is streamed through the socket
    /// if 0 bytes are read the reader exits
    pub fn stream_data<F>(&self, callback: F, buffsize: usize) -> Result<(), TorErrors>
    where
        F: DataObserver + Send + 'static,
    {
        let tcp_stream = self.stream.get_ref();
        let mut reader = BufReader::new(tcp_stream.try_clone()?);
        let mut buf = vec![b'x'; buffsize];
        let _lsner_handle = (*RUNTIME).lock().unwrap().spawn(async move {
            loop {
                match reader.read(&mut buf) {
                    Ok(size) => {
                        if size == 0 {
                            break;
                        }
                        callback.on_data(base64::encode(&buf));
                    }
                    Err(e) => callback.on_error(e.to_string()),
                }
            }
        });
        Ok(())
    }
    /// Spawns a new lsnr on the tcp stream that will call the passed callback for every new line received
    /// Note: if a empty line is read the on_error callback is called with "EOF". It is up to the user to handle this as an error (dead pipe etc..) or an expected EOF, see:
    /// https://doc.rust-lang.org/std/io/trait.Read.html#tymethod.read
    /// "This reader has reached its "end of file" and will likely no longer be able to produce bytes. Note that this does not mean that the reader will always no longer be able to produce bytes."
    /// So we break the lsner loop. Caller has to re-setup onData call or start new connection
    pub fn on_data<F>(&self, callback: F) -> Result<(), TorErrors>
    where
        F: DataObserver + Send + 'static,
    {
        let tcp_stream = self.stream.get_ref().try_clone()?;
        let mut reader = BufReader::new(tcp_stream.try_clone()?);
        let _lsner_handle = (*RUNTIME).lock().unwrap().spawn(async move {
            loop {
                let mut string_buf = String::new();
                match reader.read_line(&mut string_buf) {
                    Ok(size) => {
                        if size == 0 {
                            callback.on_error(String::from("EOF"));
                            println!("Rust:Tor:TcpStream.ondata: EOF detected for read stream, shutting down streams..");
                            // if we error out on shutdown not a biggie, just log it
                            if let Err(e) = tcp_stream.shutdown(Shutdown::Write) {
                             callback.on_error(format!("Rust:Tor:TcpStream.onData: EOF Shutdown Write: {:?}",e ));
                            }
                            if let Err(e) = tcp_stream.shutdown(Shutdown::Read) {
                             callback.on_error(format!("Rust:Tor:TcpStream.onData: EOF Shutdown Read: {:?}",e ));
                            }
                            break;
                        } else {
                            callback.on_data(string_buf)
                        }
                    }
                    Err(e) => callback.on_error(e.to_string()),
                }
            };
        });
        Ok(())
    }
    /// Sends a string over the TCP connection
    /// If supplied with an optional Duration timeout to error out of write takes longer than that
    pub fn send_data(&mut self, data: String, timeout: Option<Duration>) -> Result<(), TorErrors> {
        let tcp_stream = self.stream.get_mut();
        if timeout.is_some() {
            tcp_stream.set_write_timeout(timeout)?;
        }
        tcp_stream.write_all(data.as_bytes())?;
        Ok(())
    }
    pub fn shutdown(&mut self) -> Result<(), TorErrors> {
        self.stream.get_ref().shutdown(Shutdown::Both)?;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TorService, TorServiceParam};
    use serial_test::serial;
    use std::borrow::{Borrow, BorrowMut};
    use std::cell::RefCell;
    use std::convert::TryInto;
    use std::ops::Deref;
    use std::sync::{Arc, Mutex};

    #[test]
    #[serial(tor)]
    fn connects_with_timeout() {
        let service: TorService = TorServiceParam {
            socks_port: Some(19054),
            data_dir: String::from("/tmp/sifir_rs_sdk/"),
            bootstrap_timeout_ms: Some(45000),
        }
        .try_into()
        .unwrap();
        let mut _owned_node = service.into_owned_node();
        let target = "udfpzbte2hommnvag5f3qlouqkhvp3xybhlus2yvfeqdwlhjroe4bbyd.onion:60001";
        // Connecting over Tor takes much longer than 20ms so this should panic
        // TODO improve this test
        let connection_result =
            TcpSocksStream::new_timeout(target.into(), "127.0.0.1:19054".into(), 20);
        assert_eq!(connection_result.is_err(), true);
    }

    #[test]
    #[serial(tor)]
    fn can_send_and_observe_data() {
        let service: TorService = TorServiceParam {
            socks_port: Some(19054),
            data_dir: String::from("/tmp/sifir_rs_sdk/"),
            bootstrap_timeout_ms: Some(45000),
        }
        .try_into()
        .unwrap();
        let mut owned_node = service.into_owned_node().unwrap();
        let target = "kciybn4d4vuqvobdl2kdp3r2rudqbqvsymqwg4jomzft6m6gaibaf6yd.onion:50001";
        let msg = "{ \"id\": 1, \"method\": \"blockchain.scripthash.get_balance\", \"params\": [\"716decbe1660861c3d93906cb1d98ee68b154fd4d23aed9783859c1271b52a9c\"] }\n";

        let mut tcp_com = TcpSocksStream::new(target.into(), "127.0.0.1:19054".into()).unwrap();
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
        // setup data lsner
        tcp_com.on_data(obv).unwrap();

        tcp_com.send_data(msg.into(), None).unwrap();
        tcp_com.send_data(msg.into(), None).unwrap();
        tcp_com.send_data(msg.into(), None).unwrap();
        std::thread::sleep(std::time::Duration::from_secs(7));
        tcp_com.shutdown().unwrap();
        let call_count: u16 = *count.lock().as_deref().unwrap();
        assert_eq!(call_count, 3);
        tcp_com
            .send_data(msg.into(), None)
            .expect_err("Should error out after connection has been closed");
        std::thread::sleep(std::time::Duration::from_secs(1));
        owned_node.shutdown().unwrap();
    }
}
