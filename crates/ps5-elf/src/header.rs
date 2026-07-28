use crate::{read_u16, read_u32, read_u64};
use ps5_format::elf_constants::*;
use ps5_format::error::{ParseError, Result};

#[derive(Debug, Clone, Copy)]
pub struct ElfHeader {
    pub class: u8,
    pub endian: u8,
    pub ei_version: u8,
    pub ei_osabi: u8,
    pub ei_abi_version: u8,
    pub e_type: u16,
    pub e_machine: u16,
    pub e_version: u32,
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

        if data[base..base + 4] != ELF_MAGIC {
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

        if e_machine != EM_X86_64 {
            return Err(ParseError::NotX86_64(e_machine));
        }

        Ok(Self {
            class,
            endian,
            ei_version: data[base + EI_VERSION],
            ei_osabi: data[base + EI_OSABI],
            ei_abi_version: data[base + EI_ABIVERSION],
            e_type,
            e_machine,
            e_version: read_u32(data, base + 20),
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

    pub fn osabi_name(&self) -> &'static str {
        match self.ei_osabi {
            ELFOSABI_NONE => "UNIX System V",
            ELFOSABI_HPUX => "HP-UX",
            ELFOSABI_NETBSD => "NetBSD",
            ELFOSABI_LINUX => "Linux",
            ELFOSABI_FREEBSD => "UNIX - FreeBSD",
            ELFOSABI_OPENBSD => "OpenBSD",
            _ => "Unknown",
        }
    }

    pub fn version_name(&self) -> &'static str {
        match self.e_version {
            1 => "Current",
            _ => "Unknown",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_header_bytes(osabi: u8, abi_version: u8, e_version: u32, e_type: u16) -> Vec<u8> {
        let mut buf = vec![0u8; 64];
        buf[0..4].copy_from_slice(&ELF_MAGIC);
        buf[EI_CLASS] = ELFCLASS64;
        buf[EI_DATA] = ELFDATA2LSB;
        buf[EI_VERSION] = 1;
        buf[EI_OSABI] = osabi;
        buf[EI_ABIVERSION] = abi_version;
        buf[16..18].copy_from_slice(&e_type.to_le_bytes());
        buf[18..20].copy_from_slice(&EM_X86_64.to_le_bytes());
        buf[20..24].copy_from_slice(&e_version.to_le_bytes());
        buf[52..54].copy_from_slice(&64u16.to_le_bytes());
        buf[54..56].copy_from_slice(&56u16.to_le_bytes());
        buf
    }

    #[test]
    fn parse_header_preserves_ident_fields() {
        let data = build_header_bytes(ELFOSABI_FREEBSD, 2, 1, ET_SCE_DYNEXEC);
        let h = ElfHeader::parse(&data, 0).unwrap();

        assert_eq!(h.ei_version, 1);
        assert_eq!(h.ei_osabi, ELFOSABI_FREEBSD);
        assert_eq!(h.ei_abi_version, 2);
        assert_eq!(h.e_version, 1);
        assert_eq!(h.e_type, ET_SCE_DYNEXEC);
    }

    #[test]
    fn osabi_name_freebsd() {
        let data = build_header_bytes(ELFOSABI_FREEBSD, 0, 1, ET_SCE_DYNEXEC);
        let h = ElfHeader::parse(&data, 0).unwrap();
        assert_eq!(h.osabi_name(), "UNIX - FreeBSD");
    }

    #[test]
    fn osabi_name_system_v() {
        let data = build_header_bytes(ELFOSABI_NONE, 0, 1, ET_SCE_DYNEXEC);
        let h = ElfHeader::parse(&data, 0).unwrap();
        assert_eq!(h.osabi_name(), "UNIX System V");
    }

    #[test]
    fn osabi_name_unknown() {
        let data = build_header_bytes(0xFF, 0, 1, ET_SCE_DYNEXEC);
        let h = ElfHeader::parse(&data, 0).unwrap();
        assert_eq!(h.osabi_name(), "Unknown");
    }

    #[test]
    fn version_name_current() {
        let data = build_header_bytes(ELFOSABI_FREEBSD, 2, 1, ET_SCE_DYNEXEC);
        let h = ElfHeader::parse(&data, 0).unwrap();
        assert_eq!(h.version_name(), "Current");
    }

    #[test]
    fn version_name_unknown() {
        let data = build_header_bytes(ELFOSABI_FREEBSD, 2, 99, ET_SCE_DYNEXEC);
        let h = ElfHeader::parse(&data, 0).unwrap();
        assert_eq!(h.version_name(), "Unknown");
    }

    #[test]
    fn non_default_osabi() {
        let data = build_header_bytes(ELFOSABI_LINUX, 0, 1, ET_SCE_DYNEXEC);
        let h = ElfHeader::parse(&data, 0).unwrap();
        assert_eq!(h.ei_osabi, ELFOSABI_LINUX);
        assert_eq!(h.osabi_name(), "Linux");
    }

    #[test]
    fn ident_offset_20_is_e_version() {
        let data = build_header_bytes(ELFOSABI_NONE, 0, 0x10000040, ET_SCE_DYNAMIC);
        let h = ElfHeader::parse(&data, 0).unwrap();
        assert_eq!(h.e_version, 0x10000040);
    }
}
