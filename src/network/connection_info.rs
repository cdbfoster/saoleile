use std::collections::HashMap;
use std::time::Instant;

use crate::event::NetworkEvent;
use crate::network::PacketHeader;

const HIGH_FREQUENCY: u8 = 20;
const LOW_FREQUENCY: u8 = 10;

pub const PING_SMOOTHING: u16 = 10;

pub struct ConnectionInfo {
    pub local_sequence: u16,
    pub remote_sequence: u16,
    pub ack: u32,
    pub ping: f32,
    pub frequency: u8,
    pub send_accumulator: u128,
    pub last_response_time: Instant,
    pub unacked_events: Vec<(Vec<u16>, Instant, Box<dyn NetworkEvent>)>,
    pub incomplete_payloads: HashMap<Vec<u16>, (Instant, Vec<Option<(PacketHeader, Vec<u8>)>>)>,
    pub disconnecting: bool,
}

impl ConnectionInfo {
    pub fn new() -> Self {
        Self {
            local_sequence: 0,
            remote_sequence: 0,
            ack: 0,
            ping: 0.0,
            frequency: HIGH_FREQUENCY,
            send_accumulator: 0,
            last_response_time: Instant::now(),
            unacked_events: Vec::new(),
            incomplete_payloads: HashMap::new(),
            disconnecting: false,
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