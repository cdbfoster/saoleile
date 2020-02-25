use crate::util::FromBytes;

const MAGIC: u32 = 0xE7E7E44E;

#[derive(Clone, Debug)]
pub struct PacketHeader {
    sequence: u16,
    ack_start: u16,
    ack: u32,
    part: u8,
    total: u8,
    sizes: Vec<u16>,
}

impl PacketHeader {
    fn from_slice(mut bytes: &[u8]) -> Result<Self, String> {
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
                let count = u8::consume_next(&mut bytes)? as usize;
                let mut sizes = Vec::with_capacity(count);
                for _ in 0..count {
                    sizes.push(u16::consume_next(&mut bytes)?);
                }
                sizes
            },
        })
    }

    fn to_vec(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.size());

        bytes.extend_from_slice(&MAGIC.to_be_bytes());

        bytes.extend_from_slice(&self.sequence.to_be_bytes());
        bytes.extend_from_slice(&self.ack_start.to_be_bytes());
        bytes.extend_from_slice(&self.ack.to_be_bytes());
        bytes.push(self.part);
        bytes.push(self.total);
        bytes.push(self.sizes.len() as u8);
        for x in &self.sizes {
            bytes.extend_from_slice(&x.to_be_bytes());
        }

        bytes
    }

    fn size(&self) -> usize {
        4 + 2 + 2 + 4 + 1 + 1 + 1 + 2 * self.sizes.len()
    }
}