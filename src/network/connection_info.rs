use std::time::Instant;

use crate::event::NetworkEvent;

pub struct ConnectionInfo {
    pub local_sequence: u16,
    pub remote_sequence: u16,
    pub previous_acks: u32,
    pub ping: f32,
    pub frequency: u8,
    pub send_accumulator: u128,
    pub unacked_events: Vec<(Vec<u16>, Instant, Box<dyn NetworkEvent>)>,
}

impl ConnectionInfo {
    pub fn acked(&self, sequences: &[u16]) -> bool {
        // If any sequence number in sequences has not been acked, return false
        panic!("Unimplemented")
    }
}