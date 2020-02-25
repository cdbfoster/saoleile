use std::convert::TryInto;

pub trait FromBytes {
    fn consume_next(bytes: &mut &[u8]) -> Result<Self, String> where Self: Sized;
}

impl FromBytes for u32 {
    fn consume_next(bytes: &mut &[u8]) -> Result<Self, String> {
        let size = std::mem::size_of::<Self>();
        if size > bytes.len() {
            return Err("Unexpected EOF!".into());
        }

        let (next, remainder) = bytes.split_at(std::mem::size_of::<Self>());
        *bytes = remainder;
        Ok(Self::from_be_bytes(next.try_into().unwrap()))
    }
}

impl FromBytes for u16 {
    fn consume_next(bytes: &mut &[u8]) -> Result<Self, String> {
        let size = std::mem::size_of::<Self>();
        if size > bytes.len() {
            return Err("Unexpected EOF!".into());
        }

        let (next, remainder) = bytes.split_at(std::mem::size_of::<Self>());
        *bytes = remainder;
        Ok(Self::from_be_bytes(next.try_into().unwrap()))
    }
}

impl FromBytes for u8 {
    fn consume_next(bytes: &mut &[u8]) -> Result<Self, String> {
        let size = std::mem::size_of::<Self>();
        if size > bytes.len() {
            return Err("Unexpected EOF!".into());
        }

        let (next, remainder) = bytes.split_at(std::mem::size_of::<Self>());
        *bytes = remainder;
        Ok(Self::from_be_bytes(next.try_into().unwrap()))
    }
}