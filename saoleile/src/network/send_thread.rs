use std::collections::HashMap;
use std::i128;
use std::net::{SocketAddr, UdpSocket};
use std::sync::{Arc, mpsc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::event::NetworkEvent;

use crate::network::{MAX_PACKET_SIZE, ShutdownEvent};
use crate::network::connection_data::{ConnectionData, LOW_FREQUENCY};
use crate::network::packet_header::PacketHeader;

pub fn network_interface_send_thread(
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

pub fn send_events(socket: &UdpSocket, address: SocketAddr, local_sequence: u16, remote_sequence: u16, ack: u32, events: &[Box<dyn NetworkEvent>]) -> usize {
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