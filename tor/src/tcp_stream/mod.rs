use crate::RUNTIME;
use socks::Socks5Stream;
use std::io::BufRead;
use std::io::BufReader;
use std::io::{Read, Write};
use std::time::Duration;
use tokio::net::TcpStream;

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
    pub fn new(target: String, socks_proxy: String) -> Self {
        let socks_stream = Socks5Stream::connect(socks_proxy.as_str(), target.as_str()).unwrap();
        TcpSocksStream {
            target,
            socks_proxy,
            stream: socks_stream,
        }
    }
    /// Spawns a new lsnr on the tcp stream that will call the passed callback with bufsize bytes of
    /// base64 encoded string of data as it is streamed through the socket
    /// if 0 bytes are read the reader exits
    pub fn stream_data<F>(&self, callback: F, buffsize: usize) -> anyhow::Result<()>
    where
        F: DataObserver + Send + 'static,
    {
        let tcp_stream = self.stream.get_ref();
        let mut reader = BufReader::new(tcp_stream.try_clone()?);
        let mut buf = vec![b'x'; buffsize];
        let _lsnr_handle = (*RUNTIME).lock().unwrap().spawn(async move {
            loop {
                match reader.read(&mut buf) {
                    Ok(x) => {
                        if x < 1 {
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
    pub fn on_data<F>(&self, callback: F) -> anyhow::Result<()>
    where
        F: DataObserver + Send + 'static,
    {
        let tcp_stream = self.stream.get_ref();
        let mut reader = BufReader::new(tcp_stream.try_clone()?);
        let _lsnr_handle = (*RUNTIME).lock().unwrap().spawn(async move {
            loop {
                let mut string_buf = String::new();
                match reader.read_line(&mut string_buf) {
                    Ok(_) => {
                        if string_buf == "" {
                            callback.on_error(String::from("EOF"));
                            break;
                        } else {
                            callback.on_data(string_buf)
                        }
                    }
                    Err(e) => callback.on_error(e.to_string()),
                }
            }
        });
        Ok(())
    }
    /// Sends a string over the TCP connection
    /// If supplied with an optional Duration timeout to error out of write takes longer than that
    pub fn send_data(&mut self, data: String, timeout: Option<Duration>) -> anyhow::Result<()> {
        let tcp_stream = self.stream.get_mut();
        if timeout.is_some() {
            tcp_stream.set_write_timeout(timeout)?;
        }
        tcp_stream.write_all(data.as_bytes())?;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TorService, TorServiceParam};
    use serial_test::serial;

    #[test]
    #[serial(tor)]
    fn tcp_comm() {
        let service: TorService = TorServiceParam {
            socks_port: Some(19054),
            data_dir: String::from("/tmp/sifir_rs_sdk/"),
        }
        .into();
        let mut owned_node = service.into_owned_node();
        let target = "udfpzbte2hommnvag5f3qlouqkhvp3xybhlus2yvfeqdwlhjroe4bbyd.onion:60001";
        let msg = "{ \"id\": 1, \"method\": \"blockchain.scripthash.get_balance\", \"params\": [\"716decbe1660861c3d93906cb1d98ee68b154fd4d23aed9783859c1271b52a9c\"] }\n";

        let mut tcp_com = TcpSocksStream::new(target.into(), "127.0.0.1:19054".into());
        struct Observer {}
        impl DataObserver for Observer {
            fn on_data(&self, data: String) {
                println!("Got data {}", data);
                assert_eq!(data.contains("rpc"), true);
            }
            fn on_error(&self, data: String) {
                panic!("Got error!: {}", data);
            }
        }
        // setup data lsner
        tcp_com.on_data(Observer {}).unwrap();

        tcp_com.send_data(msg.into(), None).unwrap();
        tcp_com.send_data(msg.into(), None).unwrap();
        tcp_com.send_data(msg.into(), None).unwrap();
        std::thread::sleep(std::time::Duration::from_secs(7));
        owned_node.shutdown();
    }
}
