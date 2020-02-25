use std::collections::{HashMap, HashSet};
use std::i128;
use std::io;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::str::FromStr;
use std::sync::{Arc, mpsc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::event::NetworkEvent;
use crate::event::network::DroppedNetworkEvent;

use self::connection_info::ConnectionInfo;
use self::packet_header::PacketHeader;

pub struct NetworkInterface {
    sender: Mutex<mpsc::Sender<(SocketAddr, Box<dyn NetworkEvent>)>>,
    receiver: Mutex<mpsc::Receiver<(SocketAddr, Box<dyn NetworkEvent>)>>,
    send_thread: Mutex<Option<thread::JoinHandle<()>>>,
    receive_thread: Mutex<Option<thread::JoinHandle<()>>>,

    connections: Arc<Mutex<HashMap<SocketAddr, ConnectionInfo>>>,
    whitelist: Arc<Mutex<Option<HashSet<SocketAddr>>>>,

    socket: UdpSocket,
}

impl NetworkInterface {
    pub fn new(bind: &str) -> Self {
        let (sender, send_queue) = mpsc::channel();
        let (receive_queue, receiver) = mpsc::channel();

        let connections = Arc::new(Mutex::new(HashMap::new()));
        let whitelist = Arc::new(Mutex::new(None));

        let socket = UdpSocket::bind(bind).expect("NetworkInterface: Cannot bind to address!");

        let send_thread = Mutex::new(Some({
            let socket = socket.try_clone().expect("NetworkInterface: Cannot clone socket!");
            let receive_queue = receive_queue.clone();
            let connections = connections.clone();
            thread::spawn(move || network_interface_send_thread(socket, send_queue, receive_queue, connections))
        }));

        let receive_thread = Mutex::new(Some({
            let socket = socket.try_clone().expect("NetworkInterface: Cannot clone socket!");
            let connections = connections.clone();
            let whitelist = whitelist.clone();
            thread::spawn(move || network_interface_receive_thread(socket, receive_queue, connections, whitelist))
        }));

        Self {
            sender: Mutex::new(sender),
            receiver: Mutex::new(receiver),
            send_thread,
            receive_thread,
            connections,
            whitelist,
            socket,
        }
    }

    pub fn connect(&self, address: &str) -> Result<(), String> {
        panic!("Unimplemented")
    }
}

fn network_interface_send_thread(
    socket: UdpSocket,
    send_queue: mpsc::Receiver<(SocketAddr, Box<dyn NetworkEvent>)>,
    receive_queue: mpsc::Sender<(SocketAddr, Box<dyn NetworkEvent>)>,
    connections: Arc<Mutex<HashMap<SocketAddr, ConnectionInfo>>>,
) {
    let mut buffer: HashMap<SocketAddr, Vec<Box<dyn NetworkEvent>>> = HashMap::new();

    let mut previous_time = Instant::now();

    'send: loop {
        let current_time = Instant::now();
        let delta_time = current_time.duration_since(previous_time);
        previous_time = current_time;

        // Handle any overdue buffers
        {
            let mut connections_guard = connections.lock().unwrap();
            for (address, info) in connections_guard.iter_mut() {
                info.send_accumulator += delta_time.as_nanos();

                let iteration_ns = 1_000_000_000 as u128 / info.frequency as u128;
                if info.send_accumulator >= iteration_ns {
                    // Clear acked packets, and packets that haven't been acked in at least a second
                    let now = Instant::now();
                    let mut removed_indices = 0;
                    for i in 0..info.unacked_events.len() {
                        let current_index = i - removed_indices;
                        let event = &info.unacked_events[current_index];

                        if info.acked(&event.0) {
                            info.unacked_events.remove(current_index);
                            removed_indices += 1;
                        } else if now.duration_since(event.1).as_secs() >= 1 {
                            let (_, _, dropped_event) = info.unacked_events.remove(current_index);

                            receive_queue.send((
                                *address,
                                Box::new(DroppedNetworkEvent {
                                    recipient: *address,
                                    event: dropped_event,
                                }),
                            )).unwrap();

                            removed_indices += 1;
                        }
                    }

                    // Send events that have piled up for this address
                    let current_buffer = buffer.remove(address).unwrap_or(Vec::new());
                    send_events(&socket, *address, info, current_buffer);

                    info.send_accumulator -= iteration_ns;
                    if info.send_accumulator >= iteration_ns {
                        log!(INFO, "NetworkInterface is behind!");
                    }
                }
            }
        }

        // Gather new events
        let mut new_events = Vec::new();
        loop {
            if let Ok(event) = send_queue.try_recv() {
                if event.1.as_any().is::<ShutdownEvent>() {
                    log!("NetworkInterface send thread received a shutdown event.");
                    break 'send;
                }
                new_events.push(event);
            } else {
                break;
            }
        }

        // Get new events into buffers
        {
            let connections_guard = connections.lock().unwrap();
            for (sender, event) in new_events {
                let mut found = false;
                for (address, _) in connections_guard.iter() {
                    if sender == *address {
                        found = true;
                        break;
                    }
                }
                if !found {
                    log!(INFO, "NetworkInterface send thread: Attempting to send an event to an unconnected address!");
                    continue;
                }

                buffer.entry(sender).or_insert(Vec::new()).push(event);
            }
        }

        // Sleep till next due buffer
        let current_time = Instant::now();
        let delta_time = current_time.duration_since(previous_time);
        previous_time = current_time;
        let time_to_next_due = {
            let mut minimum_time = i128::MAX;
            let mut connections_guard = connections.lock().unwrap();
            for (_, info) in connections_guard.iter_mut() {
                info.send_accumulator += delta_time.as_nanos();
                let iteration_ns = 1_000_000_000 as u128 / info.frequency as u128;
                let remaining_time = iteration_ns as i128 - info.send_accumulator as i128;
                if remaining_time < minimum_time {
                    minimum_time = remaining_time;
                }
            }
            minimum_time
        };
        if time_to_next_due > 0 {
            thread::sleep(Duration::from_nanos((time_to_next_due / 2) as u64));
        }
    }
}

fn network_interface_receive_thread(
    socket: UdpSocket,
    receive_queue: mpsc::Sender<(SocketAddr, Box<dyn NetworkEvent>)>,
    connections: Arc<Mutex<HashMap<SocketAddr, ConnectionInfo>>>,
    whitelist: Arc<Mutex<Option<HashSet<SocketAddr>>>>,
) {
    panic!("Unimplemented")
}

fn send_events(socket: &UdpSocket, address: SocketAddr, connection_info: &mut ConnectionInfo, events: Vec<Box<dyn NetworkEvent>>) {
    panic!("Unimplemented")
}

#[derive(Deserialize, Serialize)]
struct ShutdownEvent { }

#[typetag::serde]
impl NetworkEvent for ShutdownEvent { }

mod connection_info;
mod packet_header;