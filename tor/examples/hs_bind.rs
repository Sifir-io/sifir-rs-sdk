use tor::*;

fn main() {
    println!("Sifir - Hidden Service Creator !");
    println!("This will create a hidden service that forwards incoming connections to a port of your choosing");
    println!("---------------");
    println!("Enter a port to foward onion");
    let mut port = String::new();
    std::io::stdin().read_line(&mut port).unwrap();
    let hs_port: u16 = 20011;
    let to_port: u16 = port.trim().parse::<u16>().unwrap();

    let service: TorService = TorServiceParam {
        socks_port: Some(19054),
        data_dir: String::from("/tmp/sifir_rs_sdk/"),
    }
    .into();
    let mut owned_node = service.to_owned_node(None);
    let service_key = owned_node
        .create_hidden_service(TorHiddenServiceParam { to_port, hs_port })
        .unwrap();

    let mut onion_url =
        utils::reqwest::Url::parse(&format!("http://{}", service_key.onion_url)).unwrap();
    let _ = onion_url.set_port(Some(hs_port));
    println!(
        "Hidden Service Created!\n Onion URL: {}\n Forwarding Port:{}\n",
        onion_url, to_port
    );

    println!("Press key to exit");
    std::io::stdin().read_line(&mut String::new()).unwrap();

}
