use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::sync::{Arc, mpsc, Mutex, MutexGuard};
use std::thread;

use serde::{Deserialize, Serialize};

use saoleile_derive::NetworkEvent;

use crate::event::NetworkEvent;
use crate::event::core::ShutdownEvent;

use self::cleanup_thread::network_interface_cleanup_thread;
use self::connection_data::ConnectionData;
use self::monitor_thread::network_interface_monitor_thread;
use self::packet_header::PacketHeader;
use self::receive_thread::network_interface_receive_thread;
use self::send_thread::{network_interface_send_thread, send_events};

pub const MAX_PACKET_SIZE: usize = 1024;

#[derive(Debug)]
pub struct NetworkInterface {
    sender: Mutex<mpsc::Sender<(SocketAddr, Box<dyn NetworkEvent>)>>,
    receiver: Mutex<mpsc::Receiver<(SocketAddr, Box<dyn NetworkEvent>)>>,
    cleanup: Mutex<mpsc::Sender<()>>,
    monitor: Mutex<mpsc::Sender<()>>,

    send_thread: Mutex<Option<thread::JoinHandle<()>>>,
    receive_thread: Mutex<Option<thread::JoinHandle<()>>>,
    cleanup_thread: Mutex<Option<thread::JoinHandle<()>>>,
    monitor_thread: Mutex<Option<thread::JoinHandle<()>>>,

    connections: Arc<Mutex<HashMap<SocketAddr, ConnectionData>>>,
    socket: UdpSocket,
    running: Mutex<bool>,
}

impl NetworkInterface {
    pub fn new(bind: SocketAddr) -> Self {
        let (sender, send_queue) = mpsc::channel();
        let (receive_queue, receiver) = mpsc::channel();
        let (cleanup, cleanup_receiver) = mpsc::channel();
        let (monitor, monitor_receiver) = mpsc::channel();

        let connections = Arc::new(Mutex::new(HashMap::new()));
        let socket = UdpSocket::bind(bind).expect(&format!("NetworkInterface: Cannot bind to address: {}", bind));

        let send_thread = Mutex::new(Some({
            let socket = socket.try_clone().expect("NetworkInterface: Cannot clone socket!");
            let connections = connections.clone();
            thread::Builder::new()
                .name(format!("NetworkInterface({}) send thread", socket.local_addr().unwrap()))
                .spawn(move || network_interface_send_thread(socket, send_queue, connections)).unwrap()
        }));

        let receive_thread = Mutex::new(Some({
            let socket = socket.try_clone().expect("NetworkInterface: Cannot clone socket!");
            let receive_queue = receive_queue.clone();
            let connections = connections.clone();
            thread::Builder::new()
                .name(format!("NetworkInterface({}) receive thread", socket.local_addr().unwrap()))
                .spawn(move || network_interface_receive_thread(socket, receive_queue, connections)).unwrap()
        }));

        let cleanup_thread = Mutex::new(Some({
            let connections = connections.clone();
            thread::Builder::new()
                .name(format!("NetworkInterface({}) cleanup thread", socket.local_addr().unwrap()))
                .spawn(move || network_interface_cleanup_thread(cleanup_receiver, connections, receive_queue)).unwrap()
        }));

        let monitor_thread = Mutex::new(Some({
            let connections = connections.clone();
            thread::Builder::new()
                .name(format!("NetworkInterface({}) monitor thread", socket.local_addr().unwrap()))
                .spawn(move || network_interface_monitor_thread(monitor_receiver, connections)).unwrap()
        }));

        Self {
            sender: Mutex::new(sender),
            receiver: Mutex::new(receiver),
            cleanup: Mutex::new(cleanup),
            monitor: Mutex::new(monitor),
            send_thread,
            receive_thread,
            cleanup_thread,
            monitor_thread,
            connections,
            socket,
            running: Mutex::new(true),
        }
    }

    pub fn send_event(&self, destination: SocketAddr, event: Box<dyn NetworkEvent>) {
        let sender_guard = self.sender.lock().unwrap();
        sender_guard.send((destination, event)).unwrap();
    }

    pub fn lock_receiver(&self) -> MutexGuard<mpsc::Receiver<(SocketAddr, Box<dyn NetworkEvent>)>> {
        self.receiver.lock().unwrap()
    }

    pub fn get_connection_info(&self) -> HashMap<SocketAddr, ConnectionInfo> {
        let connections_guard = self.connections.lock().unwrap();
        connections_guard.iter().map(|(&address, c)| (
            address,
            ConnectionInfo {
                address,
                ping: c.ping,
                frequency: c.frequency,
            },
        )).collect()
    }

    pub fn shutdown(&self) {
        let mut running = self.running.lock().unwrap();

        if *running {
            let this_address = self.socket.local_addr().unwrap();
            // Send a shutdown event to the send thread
            self.send_event(this_address, Box::new(ShutdownEvent { }));
            // Send a shutdown event to the receive thread
            send_events(&self.socket, this_address, 0, 0, 0, &[Box::new(ShutdownEvent { })]);
            // Send a shutdown event to the cleanup thread
            self.cleanup.lock().unwrap().send(()).unwrap();
            // Send a shutdown event to the monitor thread
            self.monitor.lock().unwrap().send(()).unwrap();

            self.send_thread.lock().unwrap().take().unwrap().join().ok();
            self.receive_thread.lock().unwrap().take().unwrap().join().ok();
            self.cleanup_thread.lock().unwrap().take().unwrap().join().ok();
            *running = false;
        }
    }
}

impl Drop for NetworkInterface {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ConnectionInfo {
    pub address: SocketAddr,
    pub ping: f32,
    pub frequency: u8,
}

mod cleanup_thread;
mod connection_data;
mod monitor_thread;
mod packet_header;
mod receive_thread;
mod send_thread;

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use std::time::Duration;

    use lazy_static::lazy_static;

    use crate::event::network::{DisconnectEvent, DroppedNetworkEvent};
    use crate::util::NetworkAbuser;
    use super::*;

    lazy_static! {
        static ref ACCESS: Mutex<()> = Mutex::new(()); // Ensures that tests run sequentially, thus not fighting over socket resources.
        static ref SERVER: SocketAddr = SocketAddr::from_str("127.0.0.1:4455").unwrap();
        static ref CLIENT: SocketAddr = SocketAddr::from_str("127.0.0.1:3333").unwrap();
        static ref CLIENT_ABUSER: SocketAddr = SocketAddr::from_str("127.0.0.1:3332").unwrap();
    }

    #[derive(Clone, Debug, Deserialize, NetworkEvent, Serialize)]
    pub struct DummyEvent { }

    #[derive(Clone, Debug, Deserialize, NetworkEvent, Serialize)]
    pub struct DummyDataEvent {
        data: Vec<u8>,
    }

    #[test]
    fn network_interface_receive_event() {
        log_level!(NONE);
        let _guard = ACCESS.lock().unwrap();
        let s = NetworkInterface::new(*SERVER);
        let c = NetworkInterface::new(*CLIENT);

        c.send_event(*SERVER, Box::new(DummyEvent { }));

        thread::sleep(Duration::from_millis(200));

        let (from, event) = s.lock_receiver().try_recv().unwrap();
        assert_eq!(from, *CLIENT);
        assert!(event.as_any().is::<DummyEvent>());
    }

    #[test]
    fn network_interface_ack() {
        log_level!(NONE);
        let _guard = ACCESS.lock().unwrap();
        let _s = NetworkInterface::new(*SERVER);
        let c = NetworkInterface::new(*CLIENT);

        c.send_event(*SERVER, Box::new(DummyEvent { }));

        thread::sleep(Duration::from_millis(200));

        assert!(c.connections.lock().unwrap().get(&*SERVER).unwrap().unacked_events.is_empty());
    }

    #[test]
    fn network_interface_dropped_event() {
        log_level!(NONE);
        let _guard = ACCESS.lock().unwrap();
        let c = NetworkInterface::new(*CLIENT);

        c.send_event(*SERVER, Box::new(DummyEvent { }));

        thread::sleep(Duration::from_millis(1200));

        let (from, event) = c.lock_receiver().try_recv().unwrap();
        assert_eq!(from, *SERVER);
        assert!(event.as_any().is::<DroppedNetworkEvent>());
    }

    #[test]
    fn network_interface_disconnect() {
        log_level!(NONE);
        let _guard = ACCESS.lock().unwrap();
        let s = NetworkInterface::new(*SERVER);
        let c = NetworkInterface::new(*CLIENT);

        c.send_event(*SERVER, Box::new(DummyEvent { }));

        thread::sleep(Duration::from_millis(200));

        c.shutdown();
        s.lock_receiver().try_recv().ok(); // Discard the event sent by the client

        thread::sleep(Duration::from_millis(8000));

        let (from, event) = s.lock_receiver().try_recv().unwrap();
        assert_eq!(from, *CLIENT);
        assert!(event.as_any().is::<DisconnectEvent>());
    }

    #[test]
    fn network_interface_send_grouped_events() {
        log_level!(NONE);
        let _guard = ACCESS.lock().unwrap();
        let c = NetworkInterface::new(*CLIENT);

        c.send_event(*SERVER, Box::new(DummyEvent { }));
        c.send_event(*SERVER, Box::new(DummyEvent { }));
        c.send_event(*SERVER, Box::new(DummyEvent { }));

        thread::sleep(Duration::from_millis(200));

        // We should have 3 unacked events
        assert_eq!(c.connections.lock().unwrap().get(&*SERVER).unwrap().unacked_events.len(), 3);
        // All unacked events should have the same sequence group: [0]
        assert!(c.connections.lock().unwrap().get(&*SERVER).unwrap().unacked_events.iter().all(|&(ref s, _, _)| s == &[0]));
    }

    #[test]
    fn network_interface_send_big_grouped_events() {
        log_level!(NONE);
        let _guard = ACCESS.lock().unwrap();
        let c = NetworkInterface::new(*CLIENT);

        c.send_event(*SERVER, Box::new(DummyEvent { }));
        c.send_event(*SERVER, Box::new(DummyDataEvent { data: vec![0xFF; 900] }));
        c.send_event(*SERVER, Box::new(DummyEvent { }));

        thread::sleep(Duration::from_millis(200));

        // We should have 3 unacked events
        assert_eq!(c.connections.lock().unwrap().get(&*SERVER).unwrap().unacked_events.len(), 3);
        // All unacked events should have the same sequence group: [0, 1]
        assert!(c.connections.lock().unwrap().get(&*SERVER).unwrap().unacked_events.iter().all(|&(ref s, _, _)| s == &[0, 1]));
    }

    #[test]
    fn network_interface_receive_big_grouped_events() {
        log_level!(NONE);
        let _guard = ACCESS.lock().unwrap();
        let s = NetworkInterface::new(*SERVER);
        let c = NetworkInterface::new(*CLIENT);

        c.send_event(*SERVER, Box::new(DummyEvent { }));
        c.send_event(*SERVER, Box::new(DummyDataEvent { data: vec![0xFF; 900] }));
        c.send_event(*SERVER, Box::new(DummyEvent { }));

        thread::sleep(Duration::from_millis(200));

        let (from, event) = s.lock_receiver().try_recv().unwrap();
        assert_eq!(from, *CLIENT);
        assert!(event.as_any().is::<DummyEvent>());

        let (from, event) = s.lock_receiver().try_recv().unwrap();
        assert_eq!(from, *CLIENT);
        assert_eq!(event.as_any().downcast_ref::<DummyDataEvent>().unwrap().data, vec![0xFF; 900]);

        let (from, event) = s.lock_receiver().try_recv().unwrap();
        assert_eq!(from, *CLIENT);
        assert!(event.as_any().is::<DummyEvent>());
    }

    #[test]
    fn network_interface_dropped_big_grouped_events() {
        // If we drop every other packet, grouped events spanning multiple packets will never make it through

        log_level!(NONE);
        let _guard = ACCESS.lock().unwrap();
        let _s = NetworkInterface::new(*SERVER);
        let c = NetworkInterface::new(*CLIENT);
        let _ca = NetworkAbuser::new(*CLIENT_ABUSER, *CLIENT, *SERVER)
            .drop_every_nth(2);

        c.send_event(*CLIENT_ABUSER, Box::new(DummyEvent { }));
        c.send_event(*CLIENT_ABUSER, Box::new(DummyDataEvent { data: vec![0xFF; 900] }));
        c.send_event(*CLIENT_ABUSER, Box::new(DummyEvent { }));

        thread::sleep(Duration::from_millis(1200));

        let (_, event) = c.lock_receiver().try_recv().unwrap();
        assert!(event.as_any().is::<DroppedNetworkEvent>());

        let (_, event) = c.lock_receiver().try_recv().unwrap();
        assert!(event.as_any().is::<DroppedNetworkEvent>());

        let (_, event) = c.lock_receiver().try_recv().unwrap();
        assert!(event.as_any().is::<DroppedNetworkEvent>());

        assert!(c.lock_receiver().try_recv().is_err());
    }
}