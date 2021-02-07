use jni_sys::*;
use serde::Serialize;
use tor::{
    tcp_stream::{DataObserver, TcpSocksStream},
    BootstrapPhase, OwnedTorService, OwnedTorServiceBootstrapPhase, TorServiceParam,
};
use std::time::Duration;

foreign_class!(class TorServiceParam {
    self_type TorServiceParam;
    constructor TorServiceParam::new(data_dir:&str,socks_port:u16)->TorServiceParam;
});
/// OwnedTorService Android Interface
foreign_class!(class OwnedTorService {
    self_type OwnedTorService;
    constructor OwnedTorService::new(param:TorServiceParam)->OwnedTorService;
    fn getSocksPort(&self)-> u16{
        this.socks_port
    }
    fn OwnedTorService::shutdown(&mut self);
    fn get_status(&mut self)-> String{
        let node_status = this.get_status();
        match node_status {
        Ok(status) => {
            let status_string = serde_json::to_string(&status).unwrap();
            println!("status is {}", status_string);
            status_string
        }
        Err(e) => {
            let message = match e.downcast::<String>() {
                Ok(msg) => msg,
                Err(_) => String::from("Unknown error"),
            };
            message
        }

    }}}

);

/// Java callback interface for TcpStream
foreign_callback!(callback DataObserver {
    self_type DataObserver;
    onData = DataObserver::on_data(&self,result: String);
    onError = DataObserver::on_error(&self,result: String);
});

// internally wrap passed the Boxed DataObserver Impl we recieve from Java
// with Observer so we can Send across threads
unsafe impl Send for Observer {}
struct Observer {
    cb: Box<dyn DataObserver>,
}
impl DataObserver for Observer {
    fn on_data(&self, data: String) {
        self.cb.on_data(data);
    }
    fn on_error(&self, data: String) {
        self.cb.on_error(data);
    }
}

/// TcpStream Android Interface
foreign_class!(class TcpSocksStream {
    self_type TcpSocksStream;
    constructor new(target:String,socks_proxy:String,timeout_ms:u64)->TcpSocksStream {
            TcpSocksStream::new_timeout(target,socks_proxy,timeout_ms).unwrap()
    }
    fn on_data(&self,cb:Box<dyn DataObserver>){
      this.on_data(Observer{
       cb,
      }).unwrap();
    }
    fn send_data(&mut self, msg:String,timeout:u64){
        this.send_data(msg, Some(Duration::new(timeout, 0))).unwrap();
        // this.send_data(msg).unwrap();
    }
});
