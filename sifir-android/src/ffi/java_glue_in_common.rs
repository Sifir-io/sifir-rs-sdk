use jni_sys::*;
use logger::{log, Logger};
use log::*;

use serde_json::json;
/// Java callback interface for DataObserver callback used in TcpStreams, HiddenService etc..
foreign_callback!(callback DataObserver {
    self_type DataObserver;
    onData = DataObserver::on_data(&self,result: String);
    onError = DataObserver::on_error(&self,result: String);
});
// internally wrap passed the Boxed DataObserver Impl we receive from Java
// with Observer so we can Send across threads
unsafe impl Send for Observer {}

unsafe impl Sync for Observer {}

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

foreign_class!(class Logger {
    self_type Logger;
    private constructor = empty;
    fn Logger::new()->Logger;
});
