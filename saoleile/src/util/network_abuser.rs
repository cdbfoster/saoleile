use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::sync::{Arc, mpsc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::network::MAX_PACKET_SIZE;

pub struct NetworkAbuser {
    running: Mutex<bool>,

    socket: UdpSocket,

    sender: Mutex<mpsc::Sender<bool>>,
    send_thread: Mutex<Option<thread::JoinHandle<()>>>,
    receive_thread: Mutex<Option<thread::JoinHandle<()>>>,

    network_conditions: Arc<Mutex<NetworkConditions>>,
}

impl NetworkAbuser {
    pub fn new(bind: SocketAddr, source: SocketAddr, target: SocketAddr) -> Self {
        let socket = UdpSocket::bind(bind).unwrap();

        let (sender, receiver) = mpsc::channel();

        let packets = Arc::new(Mutex::new(Vec::new()));
        let network_conditions = Arc::new(Mutex::new(NetworkConditions::new()));

        let receive_thread = Mutex::new(Some({
            let socket = socket.try_clone().unwrap();
            let network_conditions = network_conditions.clone();
            let sender = sender.clone();
            let packets = packets.clone();
            thread::spawn(move || network_abuser_receive_thread(socket, network_conditions, sender, source, target, packets))
        }));

        let send_thread = Mutex::new(Some({
            let socket = socket.try_clone().unwrap();
            thread::spawn(move || network_abuser_send_thread(socket, receiver, packets))
        }));

        Self {
            running: Mutex::new(true),
            socket,
            sender: Mutex::new(sender),
            send_thread,
            receive_thread,
            network_conditions,
        }
    }

    pub fn drop_every_nth(self, n: usize) -> Self {
        self.network_conditions.lock().unwrap().drop_every_nth = n;
        self
    }

    pub fn constant_latency(self, latency_ms: u64) -> Self {
        self.network_conditions.lock().unwrap().constant_latency = Duration::from_millis(latency_ms);
        self
    }

    pub fn intermittent_latency(self, latency_ms: u64, good_ms: u64, bad_ms: u64, offset_ms: i64) -> Self {
        self.network_conditions.lock().unwrap().intermittent_latencies.push((
            Duration::from_millis(latency_ms),
            Duration::from_millis(good_ms),
            Duration::from_millis(bad_ms),
            if offset_ms >= 0 {
                Instant::now() + Duration::from_millis(offset_ms as u64)
            } else {
                Instant::now() - Duration::from_millis(offset_ms.abs() as u64)
            },
        ));
        self
    }

    pub fn shutdown(&self) {
        let mut running = self.running.lock().unwrap();

        if *running {
            self.socket.send_to(&[], self.socket.local_addr().unwrap()).unwrap();
            self.receive_thread.lock().unwrap().take().unwrap().join().ok();

            self.sender.lock().unwrap().send(true).unwrap();
            self.send_thread.lock().unwrap().take().unwrap().join().ok();

            *running = false;
        }
    }
}

impl Drop for NetworkAbuser {
    fn drop(&mut self) {
        self.shutdown();
    }
}

struct NetworkConditions {
    pub drop_every_nth: usize,
    pub constant_latency: Duration,
    pub intermittent_latencies: Vec<(Duration, Duration, Duration, Instant)>,
}

impl NetworkConditions {
    pub fn new() -> Self {
        Self {
            drop_every_nth: 0,
            constant_latency: Duration::from_secs(0),
            intermittent_latencies: Vec::new(),
        }
    }
}

fn network_abuser_send_thread(
    socket: UdpSocket,
    receiver: mpsc::Receiver<bool>,
    packets: Arc<Mutex<Vec<(Instant, SocketAddr, Vec<u8>)>>>,
) {
    loop {
        let min_time = {
            let mut packets_guard = packets.lock().unwrap();

            let mut min_time = Duration::from_secs(1);

            let mut removed_indices = 0;
            for i in 0..packets_guard.len() {
                let current_index = i - removed_indices;
                let (ttl, destination, ref payload) = packets_guard[current_index];

                let now = Instant::now();
                if ttl <= now {
                    socket.send_to(payload, destination).unwrap();
                    packets_guard.remove(current_index);
                    removed_indices += 1;
                } else {
                    let remainder = ttl - now;
                    if remainder < min_time {
                        min_time = remainder;
                    }
                }
            }

            min_time
        };

        if let Ok(quit) = receiver.recv_timeout(Duration::from_nanos((min_time.as_nanos() / 2) as u64)) {
            if quit {
                break;
            }
        }
    }
}

fn network_abuser_receive_thread(
    socket: UdpSocket,
    network_conditions: Arc<Mutex<NetworkConditions>>,
    sender: mpsc::Sender<bool>,
    source: SocketAddr,
    target: SocketAddr,
    packets: Arc<Mutex<Vec<(Instant, SocketAddr, Vec<u8>)>>>,
) {
    let mut packet_numbers = HashMap::new();
    packet_numbers.insert(source, 0usize);
    packet_numbers.insert(target, 0usize);

    loop {
        let mut buffer = vec![0u8; MAX_PACKET_SIZE];

        let (size, origin) = socket.recv_from(&mut buffer).unwrap();
        buffer.truncate(size);
        let received_at = Instant::now();

        if origin == socket.local_addr().unwrap() {
            break;
        } else if !packet_numbers.contains_key(&origin) {
            continue;
        }

        let packet_number = packet_numbers.get_mut(&origin).unwrap();
        *packet_number += 1;

        let ttl = {
            let guard = network_conditions.lock().unwrap();

            if guard.drop_every_nth > 0 && *packet_number % guard.drop_every_nth == 0 {
                continue;
            }

            let now = Instant::now();

            let mut intermittent_latency = Duration::from_secs(0);
            for &(latency, good, bad, origin) in guard.intermittent_latencies.iter() {
                let cycle_length = good + bad;

                if now < origin {
                    continue;
                }

                // Duration has no modulo division.. :P
                let mut remainder = now.duration_since(origin);
                while remainder > cycle_length {
                    remainder -= cycle_length;
                }

                if remainder > good {
                    intermittent_latency += latency;
                }
            }

            received_at + guard.constant_latency + intermittent_latency
        };

        {
            let mut guard = packets.lock().unwrap();
            guard.push((
                ttl,
                if origin == source {
                    target
                } else {
                    source
                },
                buffer,
            ));
        }
        sender.send(false).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use std::thread;
    use std::time::Duration;

    use lazy_static::lazy_static;

    use super::*;

    lazy_static! {
        static ref ACCESS: Mutex<()> = Mutex::new(()); // Ensures that tests run sequentially, thus not fighting over socket resources.
        static ref ABUSER: SocketAddr = SocketAddr::from_str("127.0.0.1:1234").unwrap();
        static ref CLIENT: SocketAddr = SocketAddr::from_str("127.0.0.1:1235").unwrap();
    }

    #[test]
    fn network_abuser_drop_every_nth() {
        log_level!(NONE);
        let _guard = ACCESS.lock().unwrap();
        let _a = NetworkAbuser::new(*ABUSER, *CLIENT, *CLIENT)
            .drop_every_nth(3);
        let c = UdpSocket::bind(*CLIENT).unwrap();

        c.send_to(&[1], *ABUSER).unwrap();
        c.send_to(&[2], *ABUSER).unwrap();
        c.send_to(&[3], *ABUSER).unwrap();
        c.send_to(&[4], *ABUSER).unwrap();

        thread::sleep(Duration::from_millis(200));

        c.set_read_timeout(Some(Duration::from_millis(200))).unwrap();

        let mut buffer = [0];
        c.recv_from(&mut buffer).unwrap(); assert_eq!(buffer[0], 1);
        c.recv_from(&mut buffer).unwrap(); assert_eq!(buffer[0], 2);
        c.recv_from(&mut buffer).unwrap(); assert_eq!(buffer[0], 4);
        assert!(c.recv_from(&mut buffer).is_err());
    }
}