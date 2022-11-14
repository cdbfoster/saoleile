//use std::str::FromStr;

use saoleile::log;

fn main() {
    log!("Hello world!");

    /*let s = std::net::SocketAddr::from_str("127.0.0.1:4455").unwrap();
    let c = std::net::SocketAddr::from_str("127.0.0.1:3333").unwrap();
    let a = std::net::SocketAddr::from_str("127.0.0.1:3332").unwrap();

    let _server = saoleile::network::NetworkInterface::new(s);
    let client = saoleile::network::NetworkInterface::new(c);
    let _abuser = saoleile::util::NetworkAbuser::new(a, c, s)
        //.constant_latency(100)
        .intermittent_latency(100, 20000, 40000, -20000)
        .intermittent_latency(50, 6000, 1000, 0)
        ;

    #[derive(Debug, saoleile_derive::NetworkEvent, serde::Deserialize, serde::Serialize)]
    pub struct DummyEvent { }

    #[typetag::serde]
    impl saoleile::event::NetworkEvent for DummyEvent { }

    client.send_event(a, Box::new(DummyEvent { }));

    std::thread::sleep(std::time::Duration::from_millis(200));

    for _ in 0..1000 {
        let connections = client.get_connection_info();
        let connection_info = connections.get(&a).unwrap();
        println!("ping: {}", connection_info.ping);
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }*/

    log!("Exiting");
}