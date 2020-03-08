use std::collections::HashMap;
use std::cmp;
use std::net::SocketAddr;
use std::sync::{Arc, mpsc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::network::connection_data::{
    ConnectionData,
    HIGH_FREQUENCY,
    LATENCY_THRESHOLD,
    LOW_FREQUENCY,
    MAX_RECOVERY_COOLDOWN,
    MIN_RECOVERY_COOLDOWN,
    RECOVERY_COOLDOWN_UPDATE_PERIOD,
};

pub fn network_interface_monitor_thread(
    quit_receiver: mpsc::Receiver<()>,
    connections: Arc<Mutex<HashMap<SocketAddr, ConnectionData>>>,
) {
    log!("{} started.", thread::current().name().unwrap());

    loop {
        {
            let mut connections_guard = connections.lock().unwrap();
            for (address, connection_data) in connections_guard.iter_mut() {
                let now = Instant::now();

                let duration_since_last_change = now.duration_since(connection_data.last_frequency_change);
                let duration_since_last_update = now.duration_since(connection_data.last_cooldown_update);

                if connection_data.frequency == HIGH_FREQUENCY {
                    if connection_data.ping >= LATENCY_THRESHOLD {
                        if duration_since_last_change < RECOVERY_COOLDOWN_UPDATE_PERIOD {
                            connection_data.recovery_cooldown = cmp::min(2 * connection_data.recovery_cooldown, MAX_RECOVERY_COOLDOWN);
                            connection_data.last_cooldown_update = now;
                        }

                        log!(
                            INFO,
                            "{}: Ping to {} is high.",
                            thread::current().name().unwrap(),
                            address,
                        );

                        log!(
                            INFO,
                            "{}: ({}) Dropping send frequency to {}Hz until ping is low for at least {}s.",
                            thread::current().name().unwrap(),
                            address,
                            LOW_FREQUENCY,
                            connection_data.recovery_cooldown.as_secs(),
                        );

                        connection_data.frequency = LOW_FREQUENCY;
                        connection_data.last_frequency_change = now;
                    } else {
                        if duration_since_last_update >= RECOVERY_COOLDOWN_UPDATE_PERIOD {
                            connection_data.recovery_cooldown = cmp::max(connection_data.recovery_cooldown / 2, MIN_RECOVERY_COOLDOWN);
                            connection_data.last_cooldown_update = now;

                            log!(
                                "{}: Ping to {} has been good for a while; recovery cooldown is now {}s.",
                                thread::current().name().unwrap(),
                                address,
                                connection_data.recovery_cooldown.as_secs(),
                            );
                        }
                    }
                } else if connection_data.frequency == LOW_FREQUENCY {
                    if connection_data.ping < LATENCY_THRESHOLD {
                        if duration_since_last_change >= connection_data.recovery_cooldown {
                            log!(
                                INFO,
                                "{}: Ping to {} has improved.",
                                thread::current().name().unwrap(),
                                address,
                            );

                            log!(
                                INFO,
                                "{}: ({}) Raising send frequency to {}Hz.",
                                thread::current().name().unwrap(),
                                address,
                                HIGH_FREQUENCY,
                            );

                            connection_data.frequency = HIGH_FREQUENCY;
                            connection_data.last_frequency_change = now;
                            connection_data.last_cooldown_update = now;
                        }
                    } else {
                        connection_data.last_frequency_change = now;
                    }
                }
            }
        }

        // Sleep for half a second and quit if interrupted
        if quit_receiver.recv_timeout(Duration::from_millis(500)).is_ok() {
            log!("{} received a shutdown event.", thread::current().name().unwrap());
            break;
        }
    }

    log!("{} exiting.", thread::current().name().unwrap());
}