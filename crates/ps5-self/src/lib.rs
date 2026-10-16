use ps5_format::elf_constants::ELF_MAGIC;
use ps5_format::error::{ParseError, Result};
use ps5_format::self_constants::*;

pub mod extract;
mod segment;
#[cfg(test)]
mod tests;
pub use extract::ExtractResult;
pub use segment::SelfSegmentEntry;

fn read_u16(data: &[u8], offset: usize) -> u16 {
    if offset + 2 > data.len() {
        return 0;
    }
    u16::from_le_bytes([data[offset], data[offset + 1]])
}

fn read_u32(data: &[u8], offset: usize) -> u32 {
    if offset + 4 > data.len() {
        return 0;
    }
    u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

fn read_u64(data: &[u8], offset: usize) -> u64 {
    if offset + 8 > data.len() {
        return 0;
    }
    u64::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
        data[offset + 4],
        data[offset + 5],
        data[offset + 6],
        data[offset + 7],
    ])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelfPlatform {
    Ps4,
    Ps5,
    RawElf,
    Unknown(u32),
}

impl SelfPlatform {
    pub fn from_bytes(data: &[u8]) -> Self {
        if data.len() < 4 {
            return Self::Unknown(0);
        }
        if data[0..4] == ELF_MAGIC {
            return Self::RawElf;
        }
        let be_magic = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        match be_magic {
            SELF_MAGIC_PS4 => Self::Ps4,
            SELF_MAGIC_PS5 => Self::Ps5,
            other => Self::Unknown(other),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SelfHeader {
    pub magic: u32,
    pub version: u8,
    pub mode: u8,
    pub endian: u8,
    pub attr: u8,
    pub key_type: u32,
    pub header_size: u16,
    pub meta_size: u16,
    pub file_size: u64,
    pub num_segments: u16,
    pub flags: u16,
}

#[derive(Debug, Clone)]
pub struct SelfExtInfo {
    pub authority_id: u64,
    pub program_type: u64,
    pub app_version: u64,
    pub firmware_version: u64,
    pub digest: [u8; 32],
}

impl SelfExtInfo {
    pub fn digest_hex(&self) -> String {
        let mut s = String::with_capacity(64);
        for b in self.digest {
            s.push_str(&format!("{b:02x}"));
        }
        s
    }
}

#[derive(Debug, Clone)]
pub struct SelfImage<'a> {
    pub data: &'a [u8],
    pub platform: SelfPlatform,
    pub self_header: SelfHeader,
    pub segments: Vec<SelfSegmentEntry>,
    pub elf: ps5_elf::ElfImage<'a>,
    pub ext_info: Option<SelfExtInfo>,
}

impl<'a> SelfImage<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self> {
        if data.len() < 32 {
            return Err(ParseError::Truncated {
                offset: 0,
                needed: 32,
                available: data.len() as u64,
            });
        }

        let platform = SelfPlatform::from_bytes(data);

        if platform == SelfPlatform::RawElf {
            return Self::parse_raw_elf(data);
        }

        let magic = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);

        let self_header = SelfHeader {
            magic,
            version: data[4],
            mode: data[5],
            endian: data[6],
            attr: data[7],
            key_type: read_u32(data, 8),
            header_size: read_u16(data, 12),
            meta_size: read_u16(data, 14),
            file_size: read_u64(data, 16),
            num_segments: read_u16(data, 24),
            flags: read_u16(data, 26),
        };

        let mut segments = Vec::new();
        let seg_start = 32;
        let seg_size = 32;
        for i in 0..self_header.num_segments as usize {
            let offset = seg_start + i * seg_size;
            segments.push(SelfSegmentEntry::parse(data, offset)?);
        }

        let elf_base = seg_start + self_header.num_segments as usize * seg_size;

        // Build phdr index -> actual file offset mapping from SELF segments.
        // In a SELF file, ELF p_offset values are LOGICAL. The real data for each
        // program header lives in the SELF data segment whose (flags >> 20) == phdr_index.
        // Only segments with the data flag (0x800) carry actual ELF content.
        // SELF segment file_offset is relative to the full file, but ElfImage receives
        // &data[elf_base..], so we must subtract elf_base to get offsets relative to the ELF slice.
        let elf_slice = &data[elf_base..];
        let elf_header = ps5_elf::ElfHeader::parse(elf_slice, 0)
            .map_err(|e| ParseError::Custom(format!("ELF header parse: {e}")))?;
        let mut phdr_file_offsets = vec![0u64; elf_header.phnum as usize];
        let mut have_any = false;
        for seg in &segments {
            if !seg.is_data() {
                continue;
            }
            let phdr_idx = ((seg.flags >> 20) & 0xFFF) as usize;
            if phdr_idx < phdr_file_offsets.len() {
                phdr_file_offsets[phdr_idx] = seg.file_offset.saturating_sub(elf_base as u64);
                have_any = true;
            }
        }

        let offsets_ref = if have_any {
            Some(phdr_file_offsets.as_slice())
        } else {
            None
        };
        let elf = ps5_elf::ElfImage::parse(elf_slice, offsets_ref)?;
        let ext_info = parse_ext_info(data, elf_base, &elf);

        Ok(Self {
            data,
            platform,
            self_header,
            segments,
            elf,
            ext_info,
        })
    }

    fn parse_raw_elf(data: &'a [u8]) -> Result<Self> {
        let elf = ps5_elf::ElfImage::parse(data, None)?;
        let fake_header = SelfHeader {
            magic: 0x7F454C46,
            version: 1,
            mode: 0,
            endian: 1,
            attr: 0,
            key_type: 0,
            header_size: 0,
            meta_size: 0,
            file_size: data.len() as u64,
            num_segments: 0,
            flags: 0,
        };

        Ok(Self {
            data,
            platform: SelfPlatform::RawElf,
            self_header: fake_header,
            segments: Vec::new(),
            elf,
            ext_info: None,
        })
    }

    pub fn is_self(&self) -> bool {
        self.platform == SelfPlatform::Ps4 || self.platform == SelfPlatform::Ps5
    }

    /// SHA-256 of the extracted ELF compared against the stored extended-info
    /// digest. A mismatch does not always mean damage: a container only
    /// stores selected segments, so extraction cannot always reproduce the
    /// exact bytes that were wrapped.
    pub fn check_integrity(&self) -> Option<bool> {
        let ext = self.ext_info.as_ref()?;
        let extracted = crate::extract::extract_elf(self.data).ok()?;
        Some(ps5_format::sha256_hex(&extracted.elf) == ext.digest_hex())
    }

    pub fn data_segments(&self) -> Vec<&SelfSegmentEntry> {
        self.segments.iter().filter(|s| s.is_data()).collect()
    }
}

fn parse_ext_info(data: &[u8], elf_base: usize, elf: &ps5_elf::ElfImage) -> Option<SelfExtInfo> {
    let elf_len = 64usize.checked_add(elf.header.phnum as usize * elf.header.phentsize as usize)?;
    let ext_start = elf_base.checked_add(elf_len)?;
    let ext_start = ext_start.checked_next_multiple_of(0x10)?;
    let ext_end = ext_start.checked_add(0x40)?;
    if ext_end > data.len() {
        return None;
    }
    let header_size = read_u16(data, 12) as usize;
    if ext_end > header_size {
        return None;
    }
    let mut digest = [0u8; 32];
    digest.copy_from_slice(&data[ext_start + 0x20..ext_start + 0x40]);
    Some(SelfExtInfo {
        authority_id: read_u64(data, ext_start),
        program_type: read_u64(data, ext_start + 8),
        app_version: read_u64(data, ext_start + 16),
        firmware_version: read_u64(data, ext_start + 24),
        digest,
    })
}
