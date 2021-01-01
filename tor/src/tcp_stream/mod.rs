use crate::RUNTIME;
use socks::Socks5Stream;
use std::io::BufRead;
use std::io::BufReader;
use std::io::{Read, Write};
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
    /// Spawns a new lsnr on the tcp stream that will call the passed callback for every new line received
    /// TODO add different kinds of readers: till end, stream etc..
    pub fn on_data<F>(&self, callback: F) -> anyhow::Result<()>
    where
        F: DataObserver + Send + 'static,
    {
        let tcp_stream = self.stream.get_ref();
        let mut reader = BufReader::new(tcp_stream.try_clone()?);
        let lsnr_handle = (*RUNTIME).lock().unwrap().spawn(async move {
            loop {
                let mut string_buf = String::new();
                match reader.read_line(&mut string_buf) {
                    Ok(_) => {
                        if string_buf == ""{
                            callback.on_error(String::from("EOF"));
                            break;
                        } else {
                            callback.on_data(string_buf)
                        }
                    },
                    Err(e) => {
                        callback.on_error(e.to_string())
                    },
                }
            }
        });
        Ok(())
    }
    pub fn send_data(&mut self, param: String) -> anyhow::Result<()> {
        let tcp_stream = self.stream.get_mut();
        tcp_stream.write_all(param.as_bytes()).unwrap();
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

        tcp_com.send_data(msg.into()).unwrap();
        tcp_com.send_data(msg.into()).unwrap();
        tcp_com.send_data(msg.into()).unwrap();
        std::thread::sleep(std::time::Duration::from_secs(7));
        owned_node.shutdown();
    }
}
