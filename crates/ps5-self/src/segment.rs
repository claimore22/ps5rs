use ps5_format::error::{ParseError, Result};
use ps5_format::self_constants::*;

fn read_u64(data: &[u8], offset: usize) -> u64 {
    if offset + 8 > data.len() { return 0; }
    u64::from_le_bytes([
        data[offset], data[offset+1], data[offset+2], data[offset+3],
        data[offset+4], data[offset+5], data[offset+6], data[offset+7],
    ])
}

#[derive(Debug, Clone)]
pub struct SelfSegmentEntry {
    pub flags: u64,
    pub file_offset: u64,
    pub file_size: u64,
    pub mem_size: u64,
}

impl SelfSegmentEntry {
    pub fn parse(data: &[u8], offset: usize) -> Result<Self> {
        if data.len() < offset + 32 {
            return Err(ParseError::Truncated {
                offset: offset as u64,
                needed: 32,
                available: (data.len() - offset) as u64,
            });
        }

        Ok(Self {
            flags: read_u64(data, offset),
            file_offset: read_u64(data, offset + 8),
            file_size: read_u64(data, offset + 16),
            mem_size: read_u64(data, offset + 24),
        })
    }

    pub fn is_data(&self) -> bool {
        self.flags & SELF_SEGMENT_FLAG_DATA != 0
    }

    pub fn is_encrypted(&self) -> bool {
        self.flags & SELF_SEGMENT_FLAG_ENCRYPTED != 0
    }

    pub fn is_compressed(&self) -> bool {
        self.flags & SELF_SEGMENT_FLAG_COMPRESSED != 0
    }

    pub fn phdr_index(&self) -> u64 {
        self_segment_key(self.flags)
    }
}
