use crate::read_u64;
use ps5_format::error::Result;

#[derive(Debug, Clone)]
pub struct RelaEntry {
    pub r_offset: u64,
    pub r_info: u64,
    pub r_addend: i64,
    pub is_plt: bool,
}

impl RelaEntry {
    pub fn r_type(&self) -> u32 {
        (self.r_info & 0xffffffff) as u32
    }

    pub fn r_sym(&self) -> u32 {
        (self.r_info >> 32) as u32
    }
}

pub fn parse_all_relocs(
    data: &[u8],
    rela_offset: u64,
    rela_size: u64,
    jmprel_offset: u64,
    jmprel_size: u64,
) -> Result<Vec<RelaEntry>> {
    let mut relocs = Vec::new();

    parse_relocs(data, rela_offset, rela_size, false, &mut relocs)?;
    parse_relocs(data, jmprel_offset, jmprel_size, true, &mut relocs)?;

    Ok(relocs)
}

fn parse_relocs(
    data: &[u8],
    offset: u64,
    size: u64,
    is_plt: bool,
    out: &mut Vec<RelaEntry>,
) -> Result<()> {
    if offset == 0 || size == 0 {
        return Ok(());
    }

    let start = offset as usize;
    let end = start + size as usize;
    let data_end = data.len();

    let mut pos = start;
    while pos + 24 <= data_end.min(end) {
        out.push(RelaEntry {
            r_offset: read_u64(data, pos),
            r_info: read_u64(data, pos + 8),
            r_addend: read_u64(data, pos + 16) as i64,
            is_plt,
        });
        pos += 24;
    }

    Ok(())
}
