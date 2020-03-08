use std::collections::HashMap;
use std::i128;
use std::net::{SocketAddr, UdpSocket};
use std::sync::{Arc, mpsc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::event::NetworkEvent;
use crate::event::network::{DisconnectEvent, DroppedNetworkEvent};

use self::connection_data::{ConnectionData, LOW_FREQUENCY, PING_SMOOTHING, wrapped_distance};
use self::packet_header::PacketHeader;

pub const MAX_PACKET_SIZE: usize = 1024;

#[derive(Debug)]
pub struct NetworkInterface {
    sender: Mutex<mpsc::Sender<(SocketAddr, Box<dyn NetworkEvent>)>>,
    receiver: Mutex<mpsc::Receiver<(SocketAddr, Box<dyn NetworkEvent>)>>,
    cleanup: Mutex<mpsc::Sender<()>>,

    send_thread: Mutex<Option<thread::JoinHandle<()>>>,
    receive_thread: Mutex<Option<thread::JoinHandle<()>>>,
    cleanup_thread: Mutex<Option<thread::JoinHandle<()>>>,

    connections: Arc<Mutex<HashMap<SocketAddr, ConnectionData>>>,
    socket: UdpSocket,
    running: Mutex<bool>,
}

impl NetworkInterface {
    pub fn new(bind: SocketAddr) -> Self {
        let (sender, send_queue) = mpsc::channel();
        let (receive_queue, receiver) = mpsc::channel();
        let (cleanup, cleanup_receiver) = mpsc::channel();

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

        Self {
            sender: Mutex::new(sender),
            receiver: Mutex::new(receiver),
            cleanup: Mutex::new(cleanup),
            send_thread,
            receive_thread,
            cleanup_thread,
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

fn network_interface_send_thread(
    socket: UdpSocket,
    send_queue: mpsc::Receiver<(SocketAddr, Box<dyn NetworkEvent>)>,
    connections: Arc<Mutex<HashMap<SocketAddr, ConnectionData>>>,
) {
    log!("{} started.", thread::current().name().unwrap());

    let mut buffer: HashMap<SocketAddr, Vec<Box<dyn NetworkEvent>>> = HashMap::new();

    let mut previous_time = Instant::now();

    'send: loop {
        let time_to_next_due = {
            let mut connections_guard = connections.lock().unwrap();

            let current_time = Instant::now();
            let delta_time = current_time.duration_since(previous_time);
            previous_time = current_time;

            // Handle any overdue buffers
            for (address, connection_data) in connections_guard.iter_mut() {
                connection_data.send_accumulator += delta_time.as_nanos();

                let iteration_ns = 1_000_000_000 as u128 / connection_data.frequency as u128;
                if connection_data.send_accumulator >= iteration_ns {
                    // Send events that have piled up for this address
                    let current_buffer = buffer.remove(address).unwrap_or(Vec::new());
                    send_events_over_connection(&socket, *address, current_buffer, connection_data);

                    connection_data.send_accumulator -= iteration_ns;
                    if connection_data.send_accumulator >= iteration_ns {
                        log!(INFO, "{} is behind!", thread::current().name().unwrap());
                    }
                }
            }

            // Gather new events
            let mut new_events = Vec::new();
            loop {
                if let Ok(event) = send_queue.try_recv() {
                    if event.1.as_any().is::<ShutdownEvent>() {
                        log!("{} received a shutdown event.", thread::current().name().unwrap());
                        break 'send;
                    }
                    new_events.push(event);
                } else {
                    break;
                }
            }

            // Get new events into buffers
            for (destination, event) in new_events {
                if !connections_guard.contains_key(&destination) {
                    log!("{}: Creating connection info for {}.", thread::current().name().unwrap(), destination);
                    connections_guard.insert(destination, ConnectionData::new());
                }

                buffer.entry(destination).or_insert(Vec::new()).push(event);
            }

            let current_time = Instant::now();
            let delta_time = current_time.duration_since(previous_time);
            previous_time = current_time;

            let mut minimum_time = Duration::from_secs_f32(1.0 / LOW_FREQUENCY as f32).as_nanos() as i128;
            for (_, connection_data) in connections_guard.iter_mut() {
                connection_data.send_accumulator += delta_time.as_nanos();
                let iteration_ns = 1_000_000_000 as u128 / connection_data.frequency as u128;
                let remaining_time = iteration_ns as i128 - connection_data.send_accumulator as i128;
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

    log!("{} exiting.", thread::current().name().unwrap());
}

fn network_interface_receive_thread(
    socket: UdpSocket,
    receive_queue: mpsc::Sender<(SocketAddr, Box<dyn NetworkEvent>)>,
    connections: Arc<Mutex<HashMap<SocketAddr, ConnectionData>>>,
) {
    log!("{} started.", thread::current().name().unwrap());

    loop {
        let (sender, events) = if let Some(complete_payload) = receive_events(&socket, connections.clone()) {
            complete_payload
        } else {
            continue
        };

        if sender == socket.local_addr().unwrap() {
            if events.iter().any(|e| e.as_any().is::<ShutdownEvent>()) {
                log!("{} received a shutdown event.", thread::current().name().unwrap());
                break;
            }
        }

        for event in events {
            receive_queue.send((sender, event)).unwrap();
        }
    }

    log!("{} exiting.", thread::current().name().unwrap());
}

fn network_interface_cleanup_thread(
    quit_receiver: mpsc::Receiver<()>,
    connections: Arc<Mutex<HashMap<SocketAddr, ConnectionData>>>,
    receive_queue: mpsc::Sender<(SocketAddr, Box<dyn NetworkEvent>)>,
) {
    log!("{} started.", thread::current().name().unwrap());

    const ACK_TIMEOUT: Duration = Duration::from_secs(1);
    const PAYLOAD_TIMEOUT: Duration = Duration::from_secs(1);
    const CONNECTION_TIMEOUT: Duration = Duration::from_secs(8);

    loop {
        let mut minimum_time_to_next_due = Duration::from_secs(1).as_nanos() as i128;

        {
            let mut connections_guard = connections.lock().unwrap();
            for (address, connection_data) in connections_guard.iter_mut() {
                // Purge packets that haven't been acked in at least ACK_TIMEOUT
                let mut removed_indices = 0;
                for i in 0..connection_data.packet_times.len() {
                    let current_index = i - removed_indices;
                    let (sequence, send_time) = connection_data.packet_times[current_index];

                    let elapsed = Instant::now().duration_since(send_time);
                    if elapsed >= ACK_TIMEOUT {
                        log!("{}: Packet {} was never acked; considering it dropped.", thread::current().name().unwrap(), sequence);

                        connection_data.packet_times.remove(current_index);
                        removed_indices += 1;
                    }
                }

                // Purge events that haven't been acked in at least ACK_TIMEOUT
                let mut removed_indices = 0;
                for i in 0..connection_data.unacked_events.len() {
                    let current_index = i - removed_indices;
                    let (_, send_time, _) = connection_data.unacked_events[current_index];

                    let elapsed = Instant::now().duration_since(send_time);
                    if elapsed >= ACK_TIMEOUT {
                        let (sequences, _, dropped_event) = connection_data.unacked_events.remove(current_index);
                        log!("{}: Event in {:?} was never acked; considering it dropped.", thread::current().name().unwrap(), sequences);

                        receive_queue.send((
                            *address,
                            Box::new(DroppedNetworkEvent {
                                recipient: *address,
                                event: dropped_event,
                            }),
                        )).unwrap();

                        removed_indices += 1;
                    } else {
                        let remainder = (ACK_TIMEOUT - elapsed).as_nanos() as i128;
                        if  remainder < minimum_time_to_next_due {
                            minimum_time_to_next_due = remainder;
                        }
                    }
                }

                // Purge incomplete payloads older than PAYLOAD_TIMEOUT
                let incomplete_groups = connection_data.incomplete_payloads.keys().map(|k| k.clone()).collect::<Vec<_>>();
                for group in incomplete_groups {
                    let elapsed = Instant::now().duration_since(connection_data.incomplete_payloads.get(&group).unwrap().0);
                    if elapsed >= PAYLOAD_TIMEOUT {
                        connection_data.incomplete_payloads.remove(&group);
                        log!("{}: Packet group {:?} was not completed; considering it dropped.", thread::current().name().unwrap(), group);
                    } else {
                        let remainder = (PAYLOAD_TIMEOUT - elapsed).as_nanos() as i128;
                        if  remainder < minimum_time_to_next_due {
                            minimum_time_to_next_due = remainder;
                        }
                    }
                }
            }

            // Purge connections that have been stale for over CONNECTION_TIMEOUT
            let last_response_times = connections_guard.iter().map(|(k, v)| (*k, v.last_response_time)).collect::<Vec<_>>();
            for (address, last_response_time) in last_response_times {
                let elapsed = Instant::now().duration_since(last_response_time);
                if elapsed >= CONNECTION_TIMEOUT {
                    connections_guard.remove(&address);
                    log!("{}: Connection {} has not responded in over {} seconds; considering it disconnected.", thread::current().name().unwrap(), address, CONNECTION_TIMEOUT.as_secs());

                    receive_queue.send((
                        address,
                        Box::new(DisconnectEvent { }),
                    )).unwrap();
                } else {
                    let remainder = (CONNECTION_TIMEOUT - elapsed).as_nanos() as i128;
                    if  remainder < minimum_time_to_next_due {
                        minimum_time_to_next_due = remainder;
                    }
                }
            }
        }

        // Sleep till next due event (this is a maximum of 0.5 seconds)
        let quit = if minimum_time_to_next_due > 0 {
            quit_receiver.recv_timeout(Duration::from_nanos((minimum_time_to_next_due / 2) as u64)).is_ok()
        } else {
            quit_receiver.try_recv().is_ok()
        };

        if quit {
            log!("{} received a shutdown event.", thread::current().name().unwrap());
            break;
        }
    }

    log!("{} exiting.", thread::current().name().unwrap());
}

fn send_events_over_connection(socket: &UdpSocket, address: SocketAddr, mut events: Vec<Box<dyn NetworkEvent>>, connection_data: &mut ConnectionData) {
    let max_event_count = PacketHeader::max_event_count(MAX_PACKET_SIZE);

    if events.len() > max_event_count {
        for _ in (0..events.len()).step_by(max_event_count) {
            let overflow = events.split_off(events.len().min(max_event_count));
            send_events_over_connection(socket, address, events, connection_data);
            events = overflow;
        }
        return;
    }

    let packet_count = send_events(
        socket,
        address,
        connection_data.local_sequence,
        connection_data.remote_sequence,
        connection_data.ack,
        &events,
    );

    let sequences = (0..packet_count as u16).map(|x| connection_data.local_sequence.wrapping_add(x)).collect::<Vec<_>>();
    connection_data.local_sequence = connection_data.local_sequence.wrapping_add(packet_count as u16);

    let now = Instant::now();
    connection_data.unacked_events.extend(events.into_iter().map(|e| (sequences.clone(), now, e)));
    connection_data.packet_times.extend(sequences.into_iter().map(|s| (s, now)));
}

fn send_events(socket: &UdpSocket, address: SocketAddr, local_sequence: u16, remote_sequence: u16, ack: u32, events: &[Box<dyn NetworkEvent>]) -> usize {
    let mut payload_buffer = Vec::with_capacity(MAX_PACKET_SIZE);
    let mut event_sizes = Vec::with_capacity(events.len());
    for event in events.iter() {
        let old_size = payload_buffer.len();
        serde_cbor::to_writer(&mut payload_buffer, event).unwrap();
        event_sizes.push((payload_buffer.len() - old_size) as u16);
    }

    let packet_count = calculate_packet_count(events.len(), payload_buffer.len());

    let mut current_position = 0;
    for i in 0..packet_count {
        let header = PacketHeader {
            sequence: local_sequence + i as u16,
            ack_start: remote_sequence,
            ack: ack,
            part: i as u8,
            total: packet_count as u8,
            sizes: if i == 0 {
                event_sizes.clone()
            } else {
                Vec::with_capacity(0)
            },
        };

        let payload = &payload_buffer[current_position..(current_position + MAX_PACKET_SIZE - header.size()).min(payload_buffer.len())];

        send_packet(socket, address, header, payload);

        current_position += payload.len();
    }

    if current_position != payload_buffer.len() {
        log!(ERROR, "{}: There was a mismatch between packet count and payload bytes!", thread::current().name().unwrap());
    }

    packet_count
}

fn calculate_packet_count(event_count: usize, mut payload_size: usize) -> usize {
    let mut packet_count = 0;
    let mut data_size = 0;
    loop {
        packet_count += 1;
        let header_size = PacketHeader::size_by_events(if data_size == 0 {
            event_count
        } else {
            0
        });
        data_size += header_size;

        let next_packet_boundary = (data_size / MAX_PACKET_SIZE + 1) * MAX_PACKET_SIZE;
        let packet_payload_size = payload_size.min(next_packet_boundary - data_size);
        data_size += packet_payload_size;
        payload_size -= packet_payload_size;

        if payload_size == 0 {
            break;
        }
    }
    packet_count
}

fn send_packet(socket: &UdpSocket, destination: SocketAddr, header: PacketHeader, payload: &[u8]) {
    let mut packet = Vec::with_capacity(header.size() + payload.len());
    header.to_writer(&mut packet);
    packet.extend_from_slice(payload);
    if let Err(error) = socket.send_to(&packet, destination) {
        log!(ERROR, "{} send_packet: Could not send packet: {:?}", thread::current().name().unwrap(), error);
    }
}

fn receive_events(
    socket: &UdpSocket,
    connections: Arc<Mutex<HashMap<SocketAddr, ConnectionData>>>,
) -> Option<(SocketAddr, Vec<Box<dyn NetworkEvent>>)> {
    let mut buffer = vec![0u8; MAX_PACKET_SIZE];

    let (size, sender) = match socket.recv_from(&mut buffer) {
        Ok(packet_info) => packet_info,
        Err(error) => {
            log!(ERROR, "{}: Could not receive packet: {:?}", thread::current().name().unwrap(), error);
            return None;
        }
    };

    let from_self = sender == socket.local_addr().unwrap();

    buffer.truncate(size);
    let (header, payload) = match decode_packet(&buffer) {
        Ok(packet) => packet,
        Err(error) => {
            log!(ERROR, "{}: Received a bad header from {}: {:?}", thread::current().name().unwrap(), sender, error);
            return None;
        },
    };

    let mut connections_guard = connections.lock().unwrap();

    let decodable_events = if !from_self {
        let from_new_sender = !connections_guard.contains_key(&sender);

        // Setup new connection if necessary
        let mut connection_data = connections_guard
            .entry(sender)
            .or_insert(ConnectionData::new());

        // Offset new connection send times by half a send period
        if from_new_sender {
            connection_data.send_accumulator += 500_000_000 / connection_data.frequency as u128;
        }

        // Measure ping
        let now = Instant::now();
        let mut removed_indices = 0;
        for i in 0..connection_data.packet_times.len() {
            let current_index = i - removed_indices;
            let (sequence, send_time) = connection_data.packet_times[current_index];

            if header.acked(&[sequence]) {
                const EMA_WEIGHT: f32 = 2.0 / (PING_SMOOTHING as f32 + 1.0);
                let new_ping = (now - send_time).as_nanos() as f32 / 1_000_000.0;
                connection_data.ping = new_ping * EMA_WEIGHT + (1.0 - EMA_WEIGHT) * connection_data.ping;

                connection_data.packet_times.remove(current_index);
                removed_indices += 1;
            }
        }

        // Check for newly acked events
        let mut removed_indices = 0;
        for i in 0..connection_data.unacked_events.len() {
            let current_index = i - removed_indices;
            let (ref sequences, _, _) = connection_data.unacked_events[current_index];

            if header.acked(sequences) {
                connection_data.unacked_events.remove(current_index);
                removed_indices += 1;
            }
        }

        // Ignore the payload if we're out of ack range
        if wrapped_distance(header.sequence, connection_data.remote_sequence) >= 32 {
            return None;
        }

        connection_data.ack_sequence(header.sequence);
        connection_data.last_response_time = now;

        let sequence_group = header.get_sequence_group();
        if sequence_group.len() == 1 {
            // The whole payload is contained in this packet, so return that
            Some((header, payload))
        } else {
            // Otherwise, the payload is spread over a few packets, so collect the parts
            let packet_group = connection_data.incomplete_payloads
                .entry(sequence_group)
                .or_insert((Instant::now(), vec![None; header.total as usize]));

            if packet_group.1[header.part as usize].is_none() {
                packet_group.1[header.part as usize] = Some((header.clone(), payload));

                // If we have all the parts, join them and return the full payload
                if packet_group.1.iter().all(|p| p.is_some()) {
                    let (header, mut payload) = packet_group.1.remove(0).unwrap();
                    let additional_size = packet_group.1.iter().fold(0, |sum, p| sum + p.as_ref().unwrap().1.len());
                    payload.reserve_exact(additional_size);

                    for (_, payload_part) in packet_group.1.drain(..).map(|p| p.unwrap()) {
                        payload.extend_from_slice(&payload_part);
                    }

                    Some((header, payload))
                } else {
                    None
                }
            } else {
                log!(ERROR, "{}: Received a duplicate payload section!", thread::current().name().unwrap());
                None
            }
        }
    } else {
        Some((header, payload))
    };

    if let Some((header, payload)) = decodable_events {
        let events = decode_events(header, payload);
        if !events.is_empty() {
            Some((sender, events))
        } else {
            None
        }
    } else {
        None
    }
}

fn decode_packet(buffer: &[u8]) -> Result<(PacketHeader, Vec<u8>), String> {
    let header = PacketHeader::from_slice_beginning(buffer)?;
    let payload = buffer[header.size()..].to_vec();
    Ok((header, payload))
}

fn decode_events(header: PacketHeader, payload: Vec<u8>) -> Vec<Box<dyn NetworkEvent>> {
    let mut events = Vec::with_capacity(header.sizes.len());
    let mut position = 0;
    for &size in header.sizes.iter() {
        let range = position..position + size as usize;
        position += size as usize;
        events.push(match serde_cbor::from_slice(&payload[range]) {
            Ok(event) => event,
            Err(error) => {
                log!(ERROR, "{}: Received a bad payload: {:?}", thread::current().name().unwrap(), error);
                continue;
            },
        });
    }
    events
}

#[derive(Debug, Deserialize, Serialize)]
struct ShutdownEvent { }

#[typetag::serde]
impl NetworkEvent for ShutdownEvent { }

mod connection_data;
mod packet_header;

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use lazy_static::lazy_static;

    use crate::util::NetworkAbuser;
    use super::*;

    lazy_static! {
        static ref ACCESS: Mutex<()> = Mutex::new(()); // Ensures that tests run sequentially, thus not fighting over socket resources.
        static ref SERVER: SocketAddr = SocketAddr::from_str("127.0.0.1:4455").unwrap();
        static ref CLIENT: SocketAddr = SocketAddr::from_str("127.0.0.1:3333").unwrap();
        static ref CLIENT_ABUSER: SocketAddr = SocketAddr::from_str("127.0.0.1:3332").unwrap();
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct DummyEvent { }

    #[typetag::serde]
    impl NetworkEvent for DummyEvent { }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct DummyDataEvent {
        data: Vec<u8>,
    }

    #[typetag::serde]
    impl NetworkEvent for DummyDataEvent { }

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