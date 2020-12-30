use socks::Socks5Stream;
use socks::{ToTargetAddr};
use std::io::BufRead;
use std::io::BufReader;
use anyhow::Result;
use tokio::net::TcpStream;
use std::io::{Read, Write};
use crate::RUNTIME;

pub struct tcp_stream {
    target:String,
    socks_proxy:String,
    stream: Socks5Stream
}

impl tcp_stream {
    pub fn new(target:String,socks_proxy:String)->Self{
        let socks_stream = Socks5Stream::connect(socks_proxy.as_str(), target.as_str())
            .unwrap();
        tcp_stream {
            target,
            socks_proxy,
            stream: socks_stream,
        }
    }
    /// Spawns a new lsnr on the tcp stream that will call the passed callback for every new line received
    /// TODO add different kinds of readers: till end, stream etc..
    pub fn on_data<F>(&self,mut callback:F)-> anyhow::Result<()> where F:FnMut(String) + Send + 'static {
        let tcp_stream = self.stream.get_ref();
        let mut reader = BufReader::new(tcp_stream.try_clone()?);
        let lsnr_handle = (*RUNTIME).lock().unwrap().spawn(async move {
            loop {
                let mut string_buf = String::new();
                let _ = reader.read_line(&mut string_buf).unwrap();
                println!("READDERRR {}",string_buf);
                callback(string_buf);
            }
        });
        Ok(())
    }
    pub fn send_data(&mut self, param: String)->anyhow::Result<()>
    {
        let tcp_stream = self.stream.get_mut();
        tcp_stream.write_all(param.as_bytes()).unwrap();
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use crate::{TorService,TorServiceParam};

    #[test]
    #[serial(tor)]
    fn tcp_comm() {
        let service: TorService = TorServiceParam {
            socks_port: Some(19054),
            data_dir: String::from("/tmp/sifir_rs_sdk/"),
        }.into();
        let mut owned_node = service.into_owned_node();
        let target = "udfpzbte2hommnvag5f3qlouqkhvp3xybhlus2yvfeqdwlhjroe4bbyd.onion:60001";
        let msg = "{ \"id\": 1, \"method\": \"blockchain.scripthash.get_balance\", \"params\": [\"716decbe1660861c3d93906cb1d98ee68b154fd4d23aed9783859c1271b52a9c\"] }\n";

        let mut tcp_com = tcp_stream::new(target.into(), "127.0.0.1:19054".into());

        // setup data lsner
        tcp_com
            .on_data(|reply| {
                println!("GOT DAAAQTTAAA {}", reply);
                assert_eq!(reply.contains("rpc"), true);
            }).unwrap();

        tcp_com.send_data(msg.into()).unwrap();
        tcp_com.send_data(msg.into()).unwrap();
        tcp_com.send_data(msg.into()).unwrap();
        std::thread::sleep(std::time::Duration::from_secs(7));
        owned_node.shutdown();
    }
}
