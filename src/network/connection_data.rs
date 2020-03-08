use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::event::NetworkEvent;
use crate::network::PacketHeader;

pub const HIGH_FREQUENCY: u8 = 30;
pub const LOW_FREQUENCY: u8 = 10;
pub const LATENCY_THRESHOLD: f32 = 250.0;
pub const MIN_RECOVERY_COOLDOWN: Duration = Duration::from_secs(1);
pub const MAX_RECOVERY_COOLDOWN: Duration = Duration::from_secs(60);
pub const RECOVERY_COOLDOWN_UPDATE_PERIOD: Duration = Duration::from_secs(10);

pub const PING_SMOOTHING: u16 = 10;

#[derive(Debug)]
pub struct ConnectionData {
    pub local_sequence: u16,
    pub remote_sequence: u16,
    pub ack: u32,

    pub ping: f32,
    pub last_response_time: Instant,

    pub frequency: u8,
    pub last_frequency_change: Instant,
    pub last_cooldown_update: Instant,
    pub recovery_cooldown: Duration,

    pub send_accumulator: u128,

    pub unacked_events: Vec<(Vec<u16>, Instant, Box<dyn NetworkEvent>)>,
    pub packet_times: Vec<(u16, Instant)>,
    pub incomplete_payloads: HashMap<Vec<u16>, (Instant, Vec<Option<(PacketHeader, Vec<u8>)>>)>,
}

impl ConnectionData {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            local_sequence: 0,
            remote_sequence: 0,
            ack: 0,
            ping: 0.0,
            last_response_time: now,
            frequency: HIGH_FREQUENCY,
            last_frequency_change: now,
            last_cooldown_update: now,
            recovery_cooldown: Duration::from_secs(2),
            send_accumulator: 0,
            unacked_events: Vec::new(),
            packet_times: Vec::new(),
            incomplete_payloads: HashMap::new(),
        }
    }

    pub fn ack_sequence(&mut self, sequence: u16) {
        let distance = wrapped_distance(sequence, self.remote_sequence);

        if distance < 0 {
            // This sequence is more recent than anything we've had before
            self.remote_sequence = sequence;
            self.ack <<= distance.abs();
            self.ack_sequence(sequence);
        } else {
            let mask: u32 = 0x00000001 << distance;
            self.ack |= mask;
        }
    }
}

pub fn wrapped_distance(mut a: u16, mut b: u16) -> i16 {
    let sign = if b > a {
        1
    } else {
        std::mem::swap(&mut a, &mut b);
        -1
    };

    if b - a <= 32768 {
        sign * (b - a) as i16
    } else {
        sign * (a as i32 - 65536 + b as i32) as i16
    }
}

#[cfg(test)]
mod tests {
    use crate::network::packet_header::PacketHeader;
    use super::*;

    #[test]
    fn correct_acks() {
        let mut cd = ConnectionData::new();

        cd.ack_sequence(18);
        cd.ack_sequence(20);

        let ph = PacketHeader {
            sequence: 0,
            ack_start: cd.remote_sequence,
            ack: cd.ack,
            part: 0,
            total: 0,
            sizes: Vec::new(),
        };

        assert!(ph.acked(&[18, 20]));
        assert!(ph.acked(&[17]) == false);
        assert!(ph.acked(&[19]) == false);
        assert!(ph.acked(&[21]) == false);
    }
}