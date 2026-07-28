use crate::read_u64;
use ps5_format::error::{ParseError, Result};

#[derive(Debug, Clone)]
pub struct ElfSectionHeader {
    pub sh_name: u32,
    pub sh_type: u32,
    pub sh_flags: u64,
    pub sh_addr: u64,
    pub sh_offset: u64,
    pub sh_size: u64,
    pub sh_link: u32,
    pub sh_info: u32,
    pub sh_addralign: u64,
    pub sh_entsize: u64,
}

pub fn parse_section_headers(
    data: &[u8],
    e_shoff: u64,
    e_shnum: u16,
    e_shentsize: u16,
    _e_shstrndx: u16,
) -> Result<Vec<ElfSectionHeader>> {
    if e_shnum == 0 || e_shoff == 0 {
        return Ok(Vec::new());
    }

    let entry_size = e_shentsize as usize;
    if entry_size != 64 {
        return Err(ParseError::Custom(format!(
            "unsupported section header entry size: {entry_size}"
        )));
    }

    let mut sections = Vec::with_capacity(e_shnum as usize);
    let start = e_shoff as usize;

    for i in 0..e_shnum as usize {
        let offset = start + i * entry_size;
        if offset + entry_size > data.len() {
            break;
        }
        sections.push(ElfSectionHeader {
            sh_name: crate::read_u32(data, offset),
            sh_type: crate::read_u32(data, offset + 4),
            sh_flags: read_u64(data, offset + 8),
            sh_addr: read_u64(data, offset + 16),
            sh_offset: read_u64(data, offset + 24),
            sh_size: read_u64(data, offset + 32),
            sh_link: crate::read_u32(data, offset + 40),
            sh_info: crate::read_u32(data, offset + 44),
            sh_addralign: read_u64(data, offset + 48),
            sh_entsize: read_u64(data, offset + 56),
        });
    }

    Ok(sections)
}

pub fn resolve_section_name(data: &[u8], shstrtab_offset: u64, name_offset: u32) -> Option<String> {
    let abs = shstrtab_offset as usize + name_offset as usize;
    if abs >= data.len() {
        return None;
    }
    let slice = &data[abs..];
    let end = slice.iter().position(|&b| b == 0).unwrap_or(slice.len());
    if end == 0 {
        return None;
    }
    String::from_utf8_lossy(&slice[..end]).into_owned().into()
}

pub fn find_build_id(data: &[u8], sections: &[ElfSectionHeader]) -> Option<String> {
    let note_section = sections
        .iter()
        .find(|s| s.sh_type == ps5_format::elf_constants::SHT_NOTE)?;
    let start = note_section.sh_offset as usize;
    let end = start + note_section.sh_size as usize;
    if end > data.len() {
        return None;
    }

    let mut pos = start;
    while pos + 12 <= end {
        let namesz = crate::read_u32(data, pos) as usize;
        let descsz = crate::read_u32(data, pos + 4) as usize;
        let ntype = crate::read_u32(data, pos + 8);

        if ntype == ps5_format::elf_constants::NT_GNU_BUILD_ID && descsz > 0 {
            let name_start = pos + 12;
            let name_end = name_start + namesz;
            let desc_start = name_end.next_multiple_of(4);
            let desc_end = desc_start + descsz;
            if desc_end <= data.len() {
                let desc = &data[desc_start..desc_end];
                return Some(desc.iter().map(|b| format!("{b:02x}")).collect());
            }
        }

        let total = 12 + namesz + descsz;
        pos += total.next_multiple_of(4);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_no_sections() {
        let sections = parse_section_headers(&[0u8; 256], 0, 0, 64, 0).unwrap();
        assert!(sections.is_empty());
    }

    #[test]
    fn parse_sections_from_data() {
        let mut data = vec![0u8; 0x200];
        let shoff: u64 = 64;
        let shnum: u16 = 2;
        let shentsize: u16 = 64;

        // Section 0: SHT_NULL
        data[64..64 + 64].fill(0);

        // Section 1: SHT_PROGBITS named ".text"
        let off2 = 64 + 64;
        // sh_name at offset 0
        data[off2..off2 + 4].copy_from_slice(&0u32.to_le_bytes());
        // sh_type = SHT_PROGBITS = 1
        data[off2 + 4..off2 + 8].copy_from_slice(&1u32.to_le_bytes());
        // sh_flags = SHF_ALLOC | SHF_EXECINSTR = 0x6
        data[off2 + 8..off2 + 16].copy_from_slice(&0x6u64.to_le_bytes());
        // sh_addr
        data[off2 + 16..off2 + 24].copy_from_slice(&0x1000u64.to_le_bytes());
        // sh_offset
        data[off2 + 24..off2 + 32].copy_from_slice(&0x1000u64.to_le_bytes());
        // sh_size
        data[off2 + 32..off2 + 40].copy_from_slice(&0x200u64.to_le_bytes());

        let sections = parse_section_headers(&data, shoff, shnum, shentsize, 0).unwrap();
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[1].sh_type, 1); // SHT_PROGBITS
        assert_eq!(sections[1].sh_addr, 0x1000);
        assert_eq!(sections[1].sh_size, 0x200);
    }

    #[test]
    fn parse_section_out_of_bounds() {
        let data = vec![0u8; 64];
        let sections = parse_section_headers(&data, 1000, 1, 64, 0).unwrap();
        assert!(sections.is_empty());
    }

    #[test]
    fn resolve_section_name_valid() {
        let mut strtab = vec![0u8]; // starts with null byte
        strtab.extend_from_slice(b".text\0.rodata\0");
        let name = resolve_section_name(&strtab, 0, 1);
        assert_eq!(name.as_deref(), Some(".text"));
        let name2 = resolve_section_name(&strtab, 0, 7);
        assert_eq!(name2.as_deref(), Some(".rodata"));
    }

    #[test]
    fn resolve_section_name_out_of_bounds() {
        let strtab = vec![0u8; 4];
        let name = resolve_section_name(&strtab, 0, 100);
        assert_eq!(name, None);
    }

    #[test]
    fn build_id_from_note_section() {
        let mut data = vec![0u8; 0x200];

        // Create a .note.gnu.build-id section at offset 0x100
        let note_start = 0x100usize;
        let namesz: u32 = 4; // "GNU\0"
        let descsz: u32 = 20;
        let ntype: u32 = 3; // NT_GNU_BUILD_ID
        data[note_start..note_start + 4].copy_from_slice(&namesz.to_le_bytes());
        data[note_start + 4..note_start + 8].copy_from_slice(&descsz.to_le_bytes());
        data[note_start + 8..note_start + 12].copy_from_slice(&ntype.to_le_bytes());
        data[note_start + 12..note_start + 16].copy_from_slice(b"GNU\0");
        // desc at offset 16 (already aligned since 12+4=16)
        for i in 0..20 {
            data[note_start + 16 + i] = (i + 1) as u8;
        }

        let sections = vec![ElfSectionHeader {
            sh_name: 0,
            sh_type: ps5_format::elf_constants::SHT_NOTE,
            sh_flags: 0,
            sh_addr: 0,
            sh_offset: note_start as u64,
            sh_size: 40,
            sh_link: 0,
            sh_info: 0,
            sh_addralign: 4,
            sh_entsize: 0,
        }];

        let build_id = find_build_id(&data, &sections).unwrap();
        assert_eq!(build_id.len(), 40); // 20 bytes * 2 hex chars
        assert!(build_id.starts_with("0102030405"));
    }

    #[test]
    fn build_id_no_note_section() {
        let data = vec![0u8; 0x200];
        let sections = vec![ElfSectionHeader {
            sh_name: 0,
            sh_type: 1, // SHT_PROGBITS, not NOTE
            sh_flags: 0,
            sh_addr: 0,
            sh_offset: 0,
            sh_size: 0,
            sh_link: 0,
            sh_info: 0,
            sh_addralign: 0,
            sh_entsize: 0,
        }];
        let build_id = find_build_id(&data, &sections);
        assert!(build_id.is_none());
    }
}
