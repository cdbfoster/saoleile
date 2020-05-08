use std::io::Write;

use crate::network::connection_data::wrapped_distance;
use crate::util::FromBytes;

const MAGIC: u32 = 0xE7E7E44E;

#[derive(Clone, Debug)]
pub struct PacketHeader {
    pub sequence: u16,
    pub ack_start: u16,
    pub ack: u32,
    pub part: u8,
    pub total: u8,
    pub sizes: Vec<u16>,
}

impl PacketHeader {
    pub fn from_slice_beginning(mut bytes: &[u8]) -> Result<Self, String> {
        let magic = u32::consume_next(&mut bytes)?;
        if magic != MAGIC {
            return Err("Wrong magic number on packet!".into());
        }

        Ok(Self {
            sequence: u16::consume_next(&mut bytes)?,
            ack_start: u16::consume_next(&mut bytes)?,
            ack: u32::consume_next(&mut bytes)?,
            part: u8::consume_next(&mut bytes)?,
            total: u8::consume_next(&mut bytes)?,
            sizes: {
                let count = u16::consume_next(&mut bytes)? as usize;
                let mut sizes = Vec::with_capacity(count);
                for _ in 0..count {
                    sizes.push(u16::consume_next(&mut bytes)?);
                }
                sizes
            },
        })
    }

    pub fn to_writer<W: Write>(&self, writer: &mut W) {
        writer.write_all(&MAGIC.to_be_bytes()).unwrap();

        writer.write_all(&self.sequence.to_be_bytes()).unwrap();
        writer.write_all(&self.ack_start.to_be_bytes()).unwrap();
        writer.write_all(&self.ack.to_be_bytes()).unwrap();
        writer.write_all(&[self.part]).unwrap();
        writer.write_all(&[self.total]).unwrap();
        writer.write_all(&(self.sizes.len() as u16).to_be_bytes()).unwrap();
        for x in &self.sizes {
            writer.write_all(&x.to_be_bytes()).unwrap();
        }
    }

    pub fn size(&self) -> usize {
        PacketHeader::size_by_events(self.sizes.len())
    }

    pub fn get_sequence_group(&self) -> Vec<u16> {
        let start = self.sequence - self.part as u16;
        let end = start + self.total as u16;
        (start..end).collect()
    }

    pub fn acked(&self, sequences: &[u16]) -> bool {
        fn sequence_acked(ack_start: u16, ack: u32, sequence: u16) -> bool {
            let distance = wrapped_distance(sequence, ack_start);
            if distance < 0 || distance >= 32 {
                return false;
            }

            let mask: u32 = 0x00000001 << distance;
            (ack & mask) != 0
        }

        sequences.iter().all(|&sequence| sequence_acked(self.ack_start, self.ack, sequence))
    }

    pub fn size_by_events(event_count: usize) -> usize {
        4 + 2 + 2 + 4 + 1 + 1 + 2 + 2 * event_count
    }

    pub fn max_event_count(max_packet_size: usize) -> usize {
        (max_packet_size - Self::size_by_events(0)) / 2
    }
}