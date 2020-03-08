use std::collections::HashMap;
use std::i128;
use std::net::{SocketAddr};
use std::sync::{Arc, mpsc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::event::NetworkEvent;
use crate::event::network::{DisconnectEvent, DroppedNetworkEvent};

use crate::network::connection_data::ConnectionData;

pub fn network_interface_cleanup_thread(
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