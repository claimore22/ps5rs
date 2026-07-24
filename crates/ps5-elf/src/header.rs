use ps5_format::error::{ParseError, Result};
use crate::{read_u16, read_u32, read_u64};
use ps5_format::elf_constants::*;

#[derive(Debug, Clone, Copy)]
pub struct ElfHeader {
    pub class: u8,
    pub endian: u8,
    pub e_type: u16,
    pub e_machine: u16,
    pub e_entry: u64,
    pub e_phoff: u64,
    pub e_shoff: u64,
    pub e_flags: u32,
    pub e_ehsize: u16,
    pub phentsize: u16,
    pub phnum: u16,
    pub shentsize: u16,
    pub shnum: u16,
    pub shstrndx: u16,
}

impl ElfHeader {
    pub fn parse(data: &[u8], base: usize) -> Result<Self> {
        if data.len() < base + 64 {
            return Err(ParseError::Truncated {
                offset: base as u64,
                needed: 64,
                available: (data.len() - base) as u64,
            });
        }

        if &data[base..base+4] != &ELF_MAGIC {
            return Err(ParseError::InvalidMagic {
                expected: u32::from_le_bytes(ELF_MAGIC),
                actual: read_u32(data, base),
            });
        }

        let class = data[base + EI_CLASS];
        if class != ELFCLASS64 {
            return Err(ParseError::UnsupportedElfClass(class));
        }

        let endian = data[base + EI_DATA];
        if endian != ELFDATA2LSB {
            return Err(ParseError::UnsupportedElfEndian(endian));
        }

        let e_type = read_u16(data, base + 16);
        let e_machine = read_u16(data, base + 18);

        Ok(Self {
            class,
            endian,
            e_type,
            e_machine,
            e_entry: read_u64(data, base + 24),
            e_phoff: read_u64(data, base + 32),
            e_shoff: read_u64(data, base + 40),
            e_flags: read_u32(data, base + 48),
            e_ehsize: read_u16(data, base + 52),
            phentsize: read_u16(data, base + 54),
            phnum: read_u16(data, base + 56),
            shentsize: read_u16(data, base + 58),
            shnum: read_u16(data, base + 60),
            shstrndx: read_u16(data, base + 62),
        })
    }

    pub fn is_executable(&self) -> bool {
        self.e_type == ET_EXEC || self.e_type == ET_SCE_DYNEXEC
    }

    pub fn is_shared(&self) -> bool {
        self.e_type == ET_DYN || self.e_type == ET_SCE_DYNAMIC
    }
}
