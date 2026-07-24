use ps5_format::error::{ParseError, Result};
use crate::{read_u32, read_u64};
use ps5_format::elf_constants::*;

#[derive(Debug, Clone)]
pub struct ProgramHeader {
    pub p_type: u32,
    pub p_flags: u32,
    pub p_offset: u64,
    pub p_vaddr: u64,
    pub p_paddr: u64,
    pub p_filesz: u64,
    pub p_memsz: u64,
    pub p_align: u64,
}

impl ProgramHeader {
    pub fn parse(data: &[u8], offset: usize) -> Result<Self> {
        if data.len() < offset + 56 {
            return Err(ParseError::Truncated {
                offset: offset as u64,
                needed: 56,
                available: (data.len() - offset) as u64,
            });
        }

        Ok(Self {
            p_type: read_u32(data, offset),
            p_flags: read_u32(data, offset + 4),
            p_offset: read_u64(data, offset + 8),
            p_vaddr: read_u64(data, offset + 16),
            p_paddr: read_u64(data, offset + 24),
            p_filesz: read_u64(data, offset + 32),
            p_memsz: read_u64(data, offset + 40),
            p_align: read_u64(data, offset + 48),
        })
    }

    pub fn is_load(&self) -> bool {
        self.p_type == PT_LOAD
    }

    pub fn is_dynamic(&self) -> bool {
        self.p_type == PT_DYNAMIC
    }

    pub fn is_tls(&self) -> bool {
        self.p_type == PT_TLS
    }

    pub fn is_executable(&self) -> bool {
        self.p_flags & PF_X != 0
    }

    pub fn is_writable(&self) -> bool {
        self.p_flags & PF_W != 0
    }

    pub fn is_readable(&self) -> bool {
        self.p_flags & PF_R != 0
    }

    pub fn type_name(&self) -> &'static str {
        match self.p_type {
            PT_NULL => "NULL",
            PT_LOAD => "LOAD",
            PT_DYNAMIC => "DYNAMIC",
            PT_TLS => "TLS",
            PT_GNU_EH_FRAME => "GNU_EH_FRAME",
            PT_GNU_RELRO => "GNU_RELRO",
            0x61000000 => "SCE_DYNLIBDATA",
            0x61000001 => "SCE_PROCPARAM",
            0x61000010 => "SCE_RELRO",
            0x60000000 => "SCE_RELA",
            _ => "UNKNOWN",
        }
    }
}
