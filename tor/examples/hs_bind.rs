use std::convert::TryInto;
use tor::*;

fn main() {
    println!("---------------");
    println!("Sifir - Hidden Service and Proxy Creator !");
    println!("This will create a hidden service that forwards incoming connections to a port of your choosing");
    println!("---------------");
    let hs_port: u16 = 20011;
    let socks_port: u16 = 19054;
    let service: TorService = TorServiceParam {
        socks_port: Some(socks_port),
        data_dir: String::from("/tmp/sifir_rs_sdk/"),
        bootstrap_timeout_ms: Some(45000),
    }
    .try_into()
    .unwrap();
    println!("---------Starting Tor Daemon and Socks Port ------");
    let mut owned_node = service.into_owned_node().unwrap();
    loop {
        println!("Enter a port to foward onion:");
        let mut port = String::new();
        std::io::stdin().read_line(&mut port).unwrap();
        let to_port: u16 = port.trim().parse::<u16>().unwrap();
        let service_key = owned_node
            .create_hidden_service(TorHiddenServiceParam {
                to_port,
                hs_port,
                secret_key: None,
            })
            .unwrap();

        let mut onion_url =
            utils::reqwest::Url::parse(&format!("http://{}", service_key.onion_url)).unwrap();
        let _ = onion_url.set_port(Some(hs_port));
        println!(
        "Hidden Service Created!!\n Hidden Service Onion URL: {}\n Forwarding to Port: {}\n Socks5 Proxy: 127.0.0.1:{}\n",
        onion_url, to_port,socks_port
        );

        // TODO write keys + param to file and on open if found prompt to restore

        println!("Press \"h\" to add a new service or any other key to exit");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        match input.trim() {
            "h" => continue,
            _ => return,
        }
    }
}
