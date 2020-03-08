use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::sync::{Arc, mpsc, Mutex};
use std::thread;
use std::time::Instant;

use crate::event::NetworkEvent;

use crate::network::{MAX_PACKET_SIZE, ShutdownEvent};
use crate::network::connection_data::{
    ConnectionData,
    PING_SMOOTHING,
    wrapped_distance,
};
use crate::network::packet_header::PacketHeader;

pub fn network_interface_receive_thread(
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